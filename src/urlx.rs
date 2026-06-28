//! URL 规范化、前缀模型、URL→rel_path 映射（共用基础模块 — constitution §2 / C-1）。
//!
//! - 规范化/前缀：M1.5
//! - 确定性 URL→rel_path 映射（全量集批量、同名/query 消歧、净化、越界安全）：M1.6 / plan §6.8

use std::collections::{BTreeMap, BTreeSet};

use url::Url;

// ===== M1.5：规范化与前缀 =====

/// 规范化（用于比较/去重）：去除 fragment。
pub fn normalize(url: &Url) -> Url {
    let mut u = url.clone();
    u.set_fragment(None);
    u
}

/// 去重键：`scheme://host/path`（去 fragment、尾斜杠归一、默认忽略 query，与 §6.8 一致）。
pub fn dedup_key(url: &Url) -> String {
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or_default();
    let mut path = url.path().to_string();
    if path.len() > 1 && path.ends_with('/') {
        path.pop();
    }
    format!("{scheme}://{host}{path}")
}

/// 由首页 URL 推导默认抓取前缀（路径目录部分）。
///
/// `…/docs/intro` → `/docs/`，`…/docs/` → `/docs/`，`…/intro` → `/`。
pub fn default_prefix(start: &Url) -> String {
    let path = start.path();
    if path.ends_with('/') {
        path.to_string()
    } else {
        match path.rfind('/') {
            Some(i) => path[..=i].to_string(),
            None => "/".to_string(),
        }
    }
}

/// 是否落在抓取范围内：同 host，且路径以任一前缀开头。
pub fn in_prefix(url: &Url, host: &str, prefixes: &[String]) -> bool {
    if url.host_str() != Some(host) {
        return false;
    }
    let path = url.path();
    prefixes.iter().any(|pre| path.starts_with(pre.as_str()))
}

/// 计算从 `from_file` 到 `to_file` 的相对路径（两者均为相对输出根的镜像 `.md` 路径）。
///
/// 例：`relative_path("docs/a.md", "docs/guide/b.md")` → `"guide/b.md"`。
pub fn relative_path(from_file: &str, to_file: &str) -> String {
    let from_parts: Vec<&str> = from_file.split('/').collect();
    let from_dirs = &from_parts[..from_parts.len().saturating_sub(1)];
    let to_parts: Vec<&str> = to_file.split('/').collect();

    let mut i = 0;
    while i < from_dirs.len() && i + 1 < to_parts.len() && from_dirs[i] == to_parts[i] {
        i += 1;
    }
    let ups = from_dirs.len() - i;
    let mut parts: Vec<String> = vec!["..".to_string(); ups];
    parts.extend(to_parts[i..].iter().map(|s| s.to_string()));
    parts.join("/")
}

// ===== M1.6：URL→rel_path 映射（§6.8，C-1）=====

/// 批量计算 `url.as_str()` → 镜像 Markdown 相对路径。
///
/// 同名消歧与 query 撞名消歧需要**全量 URL 集**，故批量处理（确定性、不逃出输出目录）。
pub fn map_paths(urls: &[Url]) -> BTreeMap<String, String> {
    struct Parsed {
        key: String,
        segs: Vec<String>,
        trailing: bool,
    }

    let parsed: Vec<Parsed> = urls
        .iter()
        .map(|u| {
            let trailing = u.path().ends_with('/');
            let segs: Vec<String> = u
                .path_segments()
                .map(|it| it.filter(|s| !s.is_empty()).map(sanitize_segment).collect())
                .unwrap_or_default();
            Parsed {
                key: u.as_str().to_string(),
                segs,
                trailing,
            }
        })
        .collect();

    // 目录集合：每个 URL 的严格祖先；trailing-slash URL 自身亦为目录（步骤 4 同名消歧依据）。
    let mut dirs: BTreeSet<Vec<String>> = BTreeSet::new();
    for p in &parsed {
        for i in 1..p.segs.len() {
            dirs.insert(p.segs[..i].to_vec());
        }
        if p.trailing && !p.segs.is_empty() {
            dirs.insert(p.segs.clone());
        }
    }

    // 稳定顺序（确定性），处理 rel 冲突。
    let mut order: Vec<usize> = (0..parsed.len()).collect();
    order.sort_by(|&a, &b| parsed[a].key.cmp(&parsed[b].key));

    let mut result = BTreeMap::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for &idx in &order {
        let p = &parsed[idx];
        let is_dir = p.trailing || p.segs.is_empty() || dirs.contains(&p.segs);

        let mut rel = if is_dir {
            if p.segs.is_empty() {
                "index.md".to_string()
            } else {
                format!("{}/index.md", p.segs.join("/"))
            }
        } else {
            let (dir_segs, last) = p.segs.split_at(p.segs.len() - 1);
            let fname = file_name_for(&last[0]);
            if dir_segs.is_empty() {
                fname
            } else {
                format!("{}/{}", dir_segs.join("/"), fname)
            }
        };

        if used.contains(&rel) {
            rel = insert_suffix(&rel, &short_hash(&p.key));
        }
        used.insert(rel.clone());
        result.insert(p.key.clone(), rel);
    }
    result
}

/// 净化单个路径段：替换文件系统保留/控制字符，阻断 `.`/`..`（越界，constitution §5）。
fn sanitize_segment(seg: &str) -> String {
    if seg == "." || seg == ".." {
        return "-".to_string();
    }
    let cleaned: String = seg
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    if cleaned.is_empty() {
        "index".to_string()
    } else {
        cleaned
    }
}

/// 末段 → `<stem>.md`：已知页面扩展名替换为 `.md`，否则整体加 `.md`。
fn file_name_for(seg: &str) -> String {
    const PAGE_EXTS: [&str; 7] = ["html", "htm", "xhtml", "php", "asp", "aspx", "jsp"];
    if let Some(dot) = seg.rfind('.') {
        let ext = seg[dot + 1..].to_ascii_lowercase();
        if PAGE_EXTS.contains(&ext.as_str()) {
            return format!("{}.md", &seg[..dot]);
        }
    }
    format!("{seg}.md")
}

fn insert_suffix(rel: &str, suffix: &str) -> String {
    match rel.strip_suffix(".md") {
        Some(stem) => format!("{stem}-{suffix}.md"),
        None => format!("{rel}-{suffix}"),
    }
}

fn short_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    format!("{:08x}", h.finish() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn relative_paths() {
        assert_eq!(relative_path("docs/a.md", "docs/b.md"), "b.md");
        assert_eq!(relative_path("docs/a.md", "docs/guide/b.md"), "guide/b.md");
        assert_eq!(relative_path("docs/guide/b.md", "docs/a.md"), "../a.md");
        assert_eq!(relative_path("a.md", "b.md"), "b.md");
        assert_eq!(
            relative_path("docs/guide/x.md", "api/y.md"),
            "../../api/y.md"
        );
    }

    #[test]
    fn normalize_strips_fragment() {
        assert_eq!(
            normalize(&u("https://x.com/a#frag")).as_str(),
            "https://x.com/a"
        );
    }

    #[test]
    fn dedup_key_ignores_trailing_slash_and_query() {
        assert_eq!(dedup_key(&u("https://x.com/docs/")), "https://x.com/docs");
        assert_eq!(
            dedup_key(&u("https://x.com/docs?v=2")),
            "https://x.com/docs"
        );
    }

    #[test]
    fn default_prefix_cases() {
        assert_eq!(default_prefix(&u("https://x.com/docs/intro")), "/docs/");
        assert_eq!(default_prefix(&u("https://x.com/docs/")), "/docs/");
        assert_eq!(default_prefix(&u("https://x.com/intro")), "/");
        assert_eq!(default_prefix(&u("https://x.com/")), "/");
    }

    #[test]
    fn in_prefix_checks_host_and_path() {
        let prefixes = vec!["/docs/".to_string()];
        assert!(in_prefix(&u("https://x.com/docs/a"), "x.com", &prefixes));
        assert!(!in_prefix(&u("https://x.com/blog/a"), "x.com", &prefixes));
        assert!(!in_prefix(&u("https://y.com/docs/a"), "x.com", &prefixes));
    }

    #[test]
    fn maps_files_dirs_and_extensions() {
        let urls = [
            u("https://x.com/"),
            u("https://x.com/docs/intro.html"),
            u("https://x.com/docs/guide/"),
        ];
        let m = map_paths(&urls);
        assert_eq!(m["https://x.com/"], "index.md");
        assert_eq!(m["https://x.com/docs/intro.html"], "docs/intro.md");
        assert_eq!(m["https://x.com/docs/guide/"], "docs/guide/index.md");
    }

    #[test]
    fn same_name_disambiguation() {
        // /guide 既是页面又是 /guide/x 的父目录 → 统一 index.md
        let urls = [u("https://x.com/guide"), u("https://x.com/guide/x")];
        let m = map_paths(&urls);
        assert_eq!(m["https://x.com/guide"], "guide/index.md");
        assert_eq!(m["https://x.com/guide/x"], "guide/x.md");
    }

    #[test]
    fn query_collision_gets_suffix() {
        let urls = [u("https://x.com/p?a=1"), u("https://x.com/p?a=2")];
        let m = map_paths(&urls);
        let a = &m["https://x.com/p?a=1"];
        let b = &m["https://x.com/p?a=2"];
        assert_ne!(a, b);
        assert!(a == "p.md" || b == "p.md");
        assert!(a.ends_with(".md") && b.ends_with(".md"));
    }

    #[test]
    fn sanitizes_traversal_and_illegal() {
        assert_eq!(sanitize_segment(".."), "-");
        assert_eq!(sanitize_segment("a:b*c"), "a-b-c");
    }
}
