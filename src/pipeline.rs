//! 编排：LLM 规则分析 → robots → discover → 阶段A（抓取+提取+缓存）→ 阶段B（下载图片+改写+转换+写盘）→ 索引/bundle + 报告。
//!
//! plan §2/§7。M1：静态引擎；M2：LLM 站点级一次规则分析（无 key 回退）；M3：图片本地化 + manifest 去重；M5：robots。
//! `std::sync::Mutex` 串行写 manifest（增量原子写支持续传）。失败隔离：单页失败不中断整体。

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use futures::stream::{self, StreamExt};
use url::Url;

use crate::assets;
use crate::cli::Mode;
use crate::cli::OutputFormat;
use crate::config::Config;
use crate::convert;
use crate::discover::{self, Discovery};
use crate::error::{Error, Result};
use crate::extract::{self, ExtractOutcome, Extracted};
use crate::fetcher::Fetcher;
use crate::llm::{self, LlmClient};
use crate::report::RunReport;
use crate::rewrite;
use crate::robots::RobotsPolicy;
use crate::rules::RuleSet;
use crate::urlx;
use crate::writer::{self, Manifest, PageRecord, PageStatus};

/// 运行完整抓取流水线，返回运行报告。
pub async fn run<F: Fetcher + Sync>(fetcher: &F, config: &Config) -> Result<RunReport> {
    if config.fresh {
        let _ = std::fs::remove_file(config.out_dir.join("manifest.json"));
        let _ = std::fs::remove_dir_all(config.out_dir.join(".cache"));
    }
    let existing = Manifest::load(&config.out_dir);
    let prefixes = effective_prefixes(config);

    // robots 拉取 / 图片下载 / LLM 共用的 HTTP 客户端（与页面抓取引擎解耦）。
    let http = reqwest::Client::builder()
        .user_agent(concat!("web2doc/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Fetch(format!("http client: {e}")))?;

    // M2：有 key → LLM 站点级一次规则分析；无 key / 空壳 → 回退默认（A6 / S7 / N-5）。
    let rules = analyze_rules(fetcher, &http, config).await;

    let robots = RobotsPolicy::load(&http, &config.start_url, config.ignore_robots).await;

    let discovery = discover::discover(
        fetcher,
        &config.start_url,
        &prefixes,
        config.max_pages,
        &rules,
        &robots,
    )
    .await?;

    // rewrite 用全量映射：dedup_key → rel_path
    let page_map: BTreeMap<String, String> = discovery
        .tasks
        .iter()
        .map(|t| (urlx::dedup_key(&t.url), t.rel_path.clone()))
        .collect();

    let manifest = Arc::new(Mutex::new(build_manifest(
        config, &rules, &discovery, existing, &prefixes,
    )));

    std::fs::create_dir_all(config.out_dir.join(".cache"))?;

    stage_fetch(fetcher, config, &rules, &manifest).await;
    stage_write(config, &page_map, &manifest, &http).await;

    let guard = lock(&manifest);
    writer::write_index(&config.out_dir, &guard, config.format)?;
    if config.bundle {
        writer::write_bundle(&config.out_dir, &guard)?;
    }

    let mut warnings = collect_warnings(&discovery, &guard);
    if rules.looks_like_spa && config.mode == Mode::Static {
        warnings.push("站点疑似 SPA，建议使用 --mode dynamic 获取完整内容".to_string());
    }
    Ok(RunReport::build(&guard, discovery.tasks.len(), warnings))
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// M2：LLM 分析首页得出规则集（站点级一次）；无 key / 渲染失败 / 静态空壳 → 回退默认。
async fn analyze_rules<F: Fetcher>(
    fetcher: &F,
    http: &reqwest::Client,
    config: &Config,
) -> RuleSet {
    let Some(key) = &config.api_key else {
        tracing::info!("未配置 LLM key，使用回退默认规则");
        return RuleSet::fallback();
    };
    tracing::info!(model = %config.model, "启用 LLM 站点级规则分析");
    let Ok(home) = fetcher.render(&config.start_url).await else {
        return RuleSet::fallback();
    };
    if config.mode == Mode::Static && llm::looks_empty(&home.html) {
        return RuleSet::fallback(); // 静态空壳短路（N-5）
    }
    let client = LlmClient::new(http.clone(), &config.base_url, &config.model, key.expose());
    client.analyze(&home.html).await
}

fn collect_warnings(discovery: &Discovery, manifest: &Manifest) -> Vec<String> {
    let mut w = Vec::new();
    if discovery.robots_skipped > 0 {
        w.push(format!(
            "{} 个页面被 robots.txt 排除",
            discovery.robots_skipped
        ));
    }
    let excluded = manifest
        .pages
        .values()
        .filter(|r| r.status == PageStatus::Excluded)
        .count();
    if excluded > 0 {
        w.push(format!("{excluded} 个页面判为非正文/空壳被排除"));
    }
    w
}

fn effective_prefixes(config: &Config) -> Vec<String> {
    let mut v = match &config.prefix {
        Some(p) => vec![p.clone()],
        None => vec![urlx::default_prefix(&config.start_url)],
    };
    v.extend(config.include_prefixes.iter().cloned());
    v
}

fn build_manifest(
    config: &Config,
    rules: &RuleSet,
    discovery: &Discovery,
    existing: Option<Manifest>,
    prefixes: &[String],
) -> Manifest {
    let mut pages = BTreeMap::new();
    for task in &discovery.tasks {
        let key = urlx::dedup_key(&task.url);
        match existing.as_ref().and_then(|m| m.pages.get(&key)) {
            // 续传：沿用既有记录（rel_path/status/cache — T-3）
            Some(prev) => {
                pages.insert(key, prev.clone());
            }
            None => {
                pages.insert(
                    key.clone(),
                    PageRecord {
                        url: task.url.as_str().to_string(),
                        rel_path: task.rel_path.clone(),
                        status: PageStatus::Pending,
                        cache: None,
                        assets: vec![],
                        error: None,
                    },
                );
            }
        }
    }
    Manifest {
        root_url: config.start_url.to_string(),
        prefix: prefixes.join(","),
        rules: rules.clone(),
        baseline_total: discovery.baseline_total,
        truncated: discovery.truncated,
        nav_order: discovery.nav_order.clone(),
        pages,
        assets_seen: existing
            .as_ref()
            .map(|m| m.assets_seen.clone())
            .unwrap_or_default(),
    }
}

/// 阶段 A：并发抓取 + 提取 + 缓存；按页增量更新 manifest（失败隔离）。
async fn stage_fetch<F: Fetcher + Sync>(
    fetcher: &F,
    config: &Config,
    rules: &RuleSet,
    manifest: &Arc<Mutex<Manifest>>,
) {
    let pending: Vec<(String, Url)> = {
        let m = lock(manifest);
        m.pages
            .iter()
            .filter(|(_, r)| r.status == PageStatus::Pending)
            .filter_map(|(k, r)| Url::parse(&r.url).ok().map(|u| (k.clone(), u)))
            .collect()
    };

    let cache_dir = config.out_dir.join(".cache");
    let out_dir = config.out_dir.clone();
    let delay = Duration::from_millis(config.delay_ms);

    stream::iter(pending)
        .for_each_concurrent(config.concurrency, |(key, url)| {
            let manifest = Arc::clone(manifest);
            let cache_dir = cache_dir.clone();
            let out_dir = out_dir.clone();
            let rules = rules.clone();
            async move {
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                let result = process_fetch(fetcher, &url, &rules, &cache_dir).await;
                let mut m = lock(&manifest);
                if let Some(rec) = m.pages.get_mut(&key) {
                    match result {
                        Ok(Some(cache_file)) => {
                            rec.status = PageStatus::Fetched;
                            rec.cache = Some(cache_file);
                        }
                        Ok(None) => rec.status = PageStatus::Excluded,
                        Err(e) => {
                            rec.status = PageStatus::Failed;
                            rec.error = Some(e.to_string());
                        }
                    }
                }
                let _ = m.save_atomic(&out_dir);
            }
        })
        .await;
}

async fn process_fetch<F: Fetcher>(
    fetcher: &F,
    url: &Url,
    rules: &RuleSet,
    cache_dir: &Path,
) -> Result<Option<String>> {
    let page = fetcher.render(url).await?;
    match extract::extract(&page.html, url, rules) {
        ExtractOutcome::Content(ex) => {
            let name = cache_name(url.as_str());
            let json = serde_json::to_string(&*ex)
                .map_err(|e| Error::Extract(format!("cache serialize: {e}")))?;
            std::fs::write(cache_dir.join(&name), json)?;
            Ok(Some(name))
        }
        ExtractOutcome::Excluded(_) => Ok(None),
    }
}

/// 阶段 B：对已 Fetched 页下载图片 + 改写 + 转换 + 写盘，标记 Written（已知全量映射）。
async fn stage_write(
    config: &Config,
    page_map: &BTreeMap<String, String>,
    manifest: &Arc<Mutex<Manifest>>,
    http: &reqwest::Client,
) {
    let fetched: Vec<(String, String, String)> = {
        let m = lock(manifest);
        m.pages
            .iter()
            .filter(|(_, r)| r.status == PageStatus::Fetched)
            .filter_map(|(k, r)| r.cache.clone().map(|c| (k.clone(), r.rel_path.clone(), c)))
            .collect()
    };

    let cache_dir = config.out_dir.join(".cache");
    let assets_dir = config.out_dir.join("assets");
    let out_dir = config.out_dir.clone();
    let host = config.start_url.host_str().unwrap_or_default().to_string();
    let prefixes = effective_prefixes(config);
    let format = config.format;

    stream::iter(fetched)
        .for_each_concurrent(config.concurrency, |(key, rel, cache_file)| {
            let host = host.clone();
            let prefixes = prefixes.clone();
            let manifest = Arc::clone(manifest);
            let cache_dir = cache_dir.clone();
            let assets_dir = assets_dir.clone();
            let out_dir = out_dir.clone();
            let page_map = page_map.clone();
            async move {
                let seen_cache = { lock(&manifest).assets_seen.clone() };
                let result = process_write(
                    http,
                    &cache_dir,
                    &cache_file,
                    &rel,
                    &page_map,
                    &assets_dir,
                    &out_dir,
                    &seen_cache,
                    &host,
                    &prefixes,
                    format,
                )
                .await;
                let mut m = lock(&manifest);
                if let Some(rec) = m.pages.get_mut(&key) {
                    match result {
                        Ok((asset_rels, new_seen)) => {
                            rec.status = PageStatus::Written;
                            rec.assets = asset_rels;
                            for (src, local) in new_seen {
                                m.assets_seen.insert(src, local);
                            }
                        }
                        Err(e) => {
                            rec.status = PageStatus::Failed;
                            rec.error = Some(e.to_string());
                        }
                    }
                }
                let _ = m.save_atomic(&out_dir);
            }
        })
        .await;
}

/// 收尾单页：下载图片本地化 → 改写 → 转 MD → 写盘。返回 (已下载资源列表, 新增 assets_seen 映射)。
#[allow(clippy::too_many_arguments)]
async fn process_write(
    http: &reqwest::Client,
    cache_dir: &Path,
    cache_file: &str,
    rel: &str,
    page_map: &BTreeMap<String, String>,
    assets_dir: &Path,
    out_dir: &Path,
    asset_seen_cache: &BTreeMap<String, String>,
    host: &str,
    prefixes: &[String],
    format: OutputFormat,
) -> Result<(Vec<String>, Vec<(String, String)>)> {
    let data = std::fs::read_to_string(cache_dir.join(cache_file))?;
    let ex: Extracted =
        serde_json::from_str(&data).map_err(|e| Error::Extract(format!("cache read: {e}")))?;

    // 下载正文图片 → asset_map（src → 相对当前页本地路径），失败则不映射。
    // 先查 manifest 级去重缓存（asset_seen_cache），命中则跳过下载（避免跨页重复）。
    let mut asset_map: BTreeMap<String, String> = BTreeMap::new();
    let mut asset_rels: Vec<String> = Vec::new();
    let mut new_seen: Vec<(String, String)> = Vec::new();
    for img in &ex.images {
        if let Some(local) = asset_seen_cache.get(img) {
            asset_map.insert(img.clone(), urlx::relative_path(rel, local));
            asset_rels.push(local.clone());
            continue;
        }
        if let Ok(name) = assets::download_image(http, img, assets_dir).await {
            let local = format!("assets/{name}");
            asset_map.insert(img.clone(), urlx::relative_path(rel, &local));
            asset_rels.push(local.clone());
            new_seen.push((img.clone(), local));
        }
    }

    let rewritten = rewrite::rewrite(&ex.content_html, rel, page_map, &asset_map, host, prefixes)?;
    match format {
        OutputFormat::Md => {
            let body = convert::to_markdown(&convert::fix_tables(&rewritten))?;
            let md = if ex.title.trim().is_empty() {
                body
            } else {
                format!("# {}\n\n{}", ex.title.trim(), body)
            };
            writer::write_markdown(out_dir, rel, &md)?;
        }
        OutputFormat::Html => {
            let page = if ex.title.trim().is_empty() {
                rewritten
            } else {
                format!("<!-- title: {} -->\n{}", ex.title, rewritten)
            };
            let html_rel = rel.replace(".md", ".html");
            writer::write_file(out_dir, &html_rel, &page)?;
        }
    }
    Ok((asset_rels, new_seen))
}

fn cache_name(key: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    format!("{:016x}.json", h.finish())
}
