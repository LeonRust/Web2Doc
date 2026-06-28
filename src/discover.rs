//! 链接发现 + 基准全集（plan §6.3）：sitemap / 导航 / 前缀 BFS → 文档页判定 → 批量映射。
//!
//! 文档页判定仅用 URL 黑名单（抓取前，T-1）；正文容器二次过滤在 extract（M1.10）。
//! robots 扣除（M5）：被 `robots.txt` 禁止的页移出基准全集，计入 `robots_skipped`（R-5）。

use std::collections::{BTreeSet, VecDeque};

use scraper::{Html, Selector};
use url::Url;

use crate::error::Result;
use crate::fetcher::Fetcher;
use crate::robots::RobotsPolicy;
use crate::rules::RuleSet;
use crate::urlx;

/// 抓取任务（plan §5）。`rel_path` 由 discover 批量映射填充。
#[derive(Debug, Clone)]
pub struct PageTask {
    pub url: Url,
    pub rel_path: String,
    pub depth: u32,
}

/// 发现结果。
#[derive(Debug, Clone)]
pub struct Discovery {
    pub tasks: Vec<PageTask>,
    /// 基准全集大小（S1 分母：前缀 ∩ 文档页 ∩ robots 允许）。
    pub baseline_total: usize,
    /// 是否因 max-pages 截断（Partial）。
    pub truncated: bool,
    /// 发现/导航顺序的 URL 键（index/bundle 排序用）。
    pub nav_order: Vec<String>,
    /// 被 robots.txt 排除的页面数（warning 用）。
    pub robots_skipped: usize,
}

/// 从 sitemap XML 提取 `<loc>` URL（兼容 urlset / sitemapindex）。
pub fn parse_sitemap_locs(xml: &str) -> Vec<String> {
    let mut locs = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<loc>") {
        let after = &rest[start + "<loc>".len()..];
        match after.find("</loc>") {
            Some(end) => {
                locs.push(after[..end].trim().to_string());
                rest = &after[end + "</loc>".len()..];
            }
            None => break,
        }
    }
    locs
}

/// 用导航选择器从 HTML 提取链接（绝对化、去 fragment）。非法选择器 → 空。
pub fn extract_links(html: &str, base: &Url, nav_selector: &str) -> Vec<Url> {
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse(nav_selector) else {
        return Vec::new();
    };
    doc.select(&sel)
        .filter_map(|el| el.value().attr("href"))
        .filter_map(|href| base.join(href).ok())
        .map(|mut u| {
            u.set_fragment(None);
            u
        })
        .collect()
}

/// URL 级文档页判定（抓取前，仅看路径 — T-1）：排除明显非正文页。
pub fn is_doc_page(url: &Url) -> bool {
    const DENY: [&str; 8] = [
        "/tags/",
        "/tag/",
        "/categories/",
        "/category/",
        "/authors/",
        "/author/",
        "/search",
        "/changelog",
    ];
    let path = url.path();
    !DENY.iter().any(|d| path.contains(d))
}

/// 由候选 URL 构建基准全集 + 任务队列：前缀 ∩ 文档页 ∩ robots 允许，去重，截断，批量映射，nav_order。
pub fn build_discovery(
    candidates: Vec<Url>,
    host: &str,
    prefixes: &[String],
    max_pages: usize,
    robots: &RobotsPolicy,
) -> Discovery {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut ordered: Vec<Url> = Vec::new();
    let mut robots_skipped = 0usize;

    for u in candidates {
        let nu = urlx::normalize(&u);
        if !urlx::in_prefix(&nu, host, prefixes) || !is_doc_page(&nu) {
            continue;
        }
        if !seen.insert(urlx::dedup_key(&nu)) {
            continue; // 去重
        }
        if !robots.is_allowed(&nu) {
            robots_skipped += 1;
            continue;
        }
        ordered.push(nu);
    }

    let baseline_total = ordered.len();
    let truncated = baseline_total > max_pages;
    if ordered.len() > max_pages {
        ordered.truncate(max_pages);
    }

    let map = urlx::map_paths(&ordered);
    let nav_order: Vec<String> = ordered.iter().map(urlx::dedup_key).collect();
    let tasks = ordered
        .into_iter()
        .map(|u| {
            let rel_path = map.get(u.as_str()).cloned().unwrap_or_default();
            PageTask {
                url: u,
                rel_path,
                depth: 0,
            }
        })
        .collect();

    Discovery {
        tasks,
        baseline_total,
        truncated,
        nav_order,
        robots_skipped,
    }
}

/// 发现编排：sitemap 优先 → 无则首页导航 + 前缀内 BFS（plan §6.3 降级链）。
pub async fn discover<F: Fetcher>(
    fetcher: &F,
    start: &Url,
    prefixes: &[String],
    max_pages: usize,
    rules: &RuleSet,
    robots: &RobotsPolicy,
) -> Result<Discovery> {
    let host = start.host_str().unwrap_or_default().to_string();

    // 1) sitemap 优先
    let mut sitemap_candidates: Vec<Url> = Vec::new();
    if let Ok(sm) = start.join("/sitemap.xml") {
        if let Ok(page) = fetcher.render(&sm).await {
            if page.status == 200 {
                sitemap_candidates = parse_sitemap_locs(&page.html)
                    .iter()
                    .filter_map(|s| Url::parse(s).ok())
                    .collect();
            }
        }
    }
    let mut discovery = build_discovery(sitemap_candidates, &host, prefixes, max_pages, robots);

    // 2) sitemap 未覆盖抓取前缀（baseline 为空）→ 回退首页导航 + 前缀内 BFS
    if discovery.baseline_total == 0 {
        let bfs = bfs_links(fetcher, start, &host, prefixes, max_pages, rules, robots).await;
        discovery = build_discovery(bfs, &host, prefixes, max_pages, robots);
    }

    Ok(discovery)
}

/// 前缀内广度优先收集链接（无 sitemap 时）。软上限防止无界扩散；被 robots 禁的不入队。
async fn bfs_links<F: Fetcher>(
    fetcher: &F,
    start: &Url,
    host: &str,
    prefixes: &[String],
    max_pages: usize,
    rules: &RuleSet,
    robots: &RobotsPolicy,
) -> Vec<Url> {
    let cap = max_pages.saturating_mul(2).max(1);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<Url> = Vec::new();
    let mut queue: VecDeque<Url> = VecDeque::new();

    seen.insert(urlx::dedup_key(start));
    out.push(start.clone());
    queue.push_back(start.clone());

    while let Some(u) = queue.pop_front() {
        if out.len() >= cap {
            break;
        }
        let Ok(page) = fetcher.render(&u).await else {
            continue;
        };
        for link in extract_links(&page.html, &u, &rules.nav_link_selector) {
            if urlx::in_prefix(&link, host, prefixes)
                && is_doc_page(&link)
                && robots.is_allowed(&link)
                && seen.insert(urlx::dedup_key(&link))
            {
                out.push(link.clone());
                queue.push_back(link);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::{Engine, RenderedPage};
    use std::collections::HashMap;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn allow_all() -> RobotsPolicy {
        RobotsPolicy::allow_all()
    }

    struct MockFetcher {
        pages: HashMap<String, (u16, String)>,
    }

    impl Fetcher for MockFetcher {
        async fn render(&self, url: &Url) -> Result<RenderedPage> {
            match self.pages.get(url.as_str()) {
                Some((status, html)) => Ok(RenderedPage {
                    final_url: url.clone(),
                    html: html.clone(),
                    status: *status,
                }),
                None => Err(crate::error::Error::Fetch(format!("not found: {url}"))),
            }
        }
        fn engine(&self) -> Engine {
            Engine::Static
        }
    }

    #[test]
    fn parses_sitemap_locs() {
        let xml = "<urlset><url><loc>https://x.com/a</loc></url><url><loc> https://x.com/b </loc></url></urlset>";
        assert_eq!(
            parse_sitemap_locs(xml),
            vec!["https://x.com/a", "https://x.com/b"]
        );
    }

    #[test]
    fn doc_page_filter() {
        assert!(is_doc_page(&u("https://x.com/docs/intro")));
        assert!(!is_doc_page(&u("https://x.com/tags/rust")));
        assert!(!is_doc_page(&u("https://x.com/search")));
    }

    #[test]
    fn extracts_nav_links() {
        let html = r#"<nav><a href="/docs/a">A</a><a href="https://x.com/docs/b">B</a></nav><a href="/other">x</a>"#;
        let links = extract_links(html, &u("https://x.com/docs/"), "nav a");
        assert_eq!(links.len(), 2);
        assert!(links.iter().any(|l| l.path() == "/docs/a"));
    }

    #[test]
    fn builds_with_filter_dedup_and_truncate() {
        let cands = vec![
            u("https://x.com/docs/a"),
            u("https://x.com/docs/a#frag"), // 去重（同页）
            u("https://x.com/docs/b"),
            u("https://x.com/tags/t"), // 文档页过滤
            u("https://y.com/docs/c"), // 前缀(host)过滤
        ];
        let d = build_discovery(cands, "x.com", &["/docs/".to_string()], 10, &allow_all());
        assert_eq!(d.baseline_total, 2);
        assert!(!d.truncated);
        assert_eq!(d.tasks.len(), 2);
        assert!(d.tasks.iter().all(|t| !t.rel_path.is_empty()));

        // 截断
        let many: Vec<Url> = (0..5)
            .map(|i| u(&format!("https://x.com/docs/p{i}")))
            .collect();
        let d2 = build_discovery(many, "x.com", &["/docs/".to_string()], 3, &allow_all());
        assert_eq!(d2.baseline_total, 5);
        assert!(d2.truncated);
        assert_eq!(d2.tasks.len(), 3);
    }

    #[test]
    fn robots_disallow_excludes_from_baseline() {
        let robots = RobotsPolicy::from_txt("User-agent: *\nDisallow: /docs/secret");
        let cands = vec![u("https://x.com/docs/a"), u("https://x.com/docs/secret/x")];
        let d = build_discovery(cands, "x.com", &["/docs/".to_string()], 10, &robots);
        assert_eq!(d.baseline_total, 1);
        assert_eq!(d.robots_skipped, 1);
        assert!(d.tasks.iter().all(|t| t.url.path() == "/docs/a"));
    }

    #[tokio::test]
    async fn discover_prefers_sitemap_and_filters() {
        let mut pages = HashMap::new();
        pages.insert(
            "https://x.com/sitemap.xml".to_string(),
            (
                200,
                "<urlset><url><loc>https://x.com/docs/a</loc></url><url><loc>https://x.com/docs/b</loc></url><url><loc>https://x.com/tags/t</loc></url></urlset>".to_string(),
            ),
        );
        let f = MockFetcher { pages };
        let d = discover(
            &f,
            &u("https://x.com/docs/"),
            &["/docs/".to_string()],
            500,
            &RuleSet::fallback(),
            &allow_all(),
        )
        .await
        .unwrap();
        assert_eq!(d.baseline_total, 2); // tags 被过滤
        assert!(d.tasks.iter().any(|t| t.url.path() == "/docs/a"));
        assert!(d.tasks.iter().any(|t| t.url.path() == "/docs/b"));
    }

    #[tokio::test]
    async fn falls_back_to_bfs_when_sitemap_misses_prefix() {
        let mut pages = HashMap::new();
        pages.insert(
            "https://x.com/sitemap.xml".to_string(),
            (
                200,
                "<urlset><url><loc>https://x.com/other/p</loc></url></urlset>".to_string(),
            ),
        );
        pages.insert(
            "https://x.com/docs/".to_string(),
            (200, r#"<nav><a href="/docs/a">A</a></nav>"#.to_string()),
        );
        pages.insert(
            "https://x.com/docs/a".to_string(),
            (200, "<p>leaf</p>".to_string()),
        );
        let f = MockFetcher { pages };
        let d = discover(
            &f,
            &u("https://x.com/docs/"),
            &["/docs/".to_string()],
            500,
            &RuleSet::fallback(),
            &allow_all(),
        )
        .await
        .unwrap();
        assert!(d.baseline_total >= 1);
        assert!(d.tasks.iter().any(|t| t.url.path() == "/docs/a"));
    }
}
