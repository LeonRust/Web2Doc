//! 链接 / 资源改写（唯一改写出口 — plan §6.7 / T-5）。
//!
//! M1：内链指向已抓本地页 → 相对化；图片 / 视频 / iframe 等保持绝对（图片本地化在 M3）。

use std::collections::BTreeMap;

use lol_html::html_content::Element;
use lol_html::{element, rewrite_str, RewriteStrSettings};
use url::Url;

use crate::error::{Error, Result};
use crate::urlx;

/// 改写正文 HTML。`page_map`：`dedup_key(绝对 URL)` → 镜像 `rel_path`。
pub fn rewrite(
    content_html: &str,
    current_rel: &str,
    page_map: &BTreeMap<String, String>,
) -> Result<String> {
    let settings = RewriteStrSettings::new().append_element_content_handler(element!(
        "a[href]",
        |el: &mut Element| {
            if let Some(href) = el.get_attribute("href") {
                if let Some(rel) = remap_internal_link(&href, current_rel, page_map) {
                    let _ = el.set_attribute("href", &rel);
                }
            }
            Ok(())
        }
    ));
    rewrite_str(content_html, settings).map_err(|e| Error::Extract(format!("rewrite: {e}")))
}

/// 若 `href` 指向已抓本地页，返回相对当前页的本地路径；否则 `None`（保持原绝对地址）。
fn remap_internal_link(
    href: &str,
    current_rel: &str,
    page_map: &BTreeMap<String, String>,
) -> Option<String> {
    let abs = Url::parse(href).ok()?;
    let target_rel = page_map.get(&urlx::dedup_key(&abs))?;
    Some(urlx::relative_path(current_rel, target_rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relativizes_internal_link_keeps_external() {
        let mut map = BTreeMap::new();
        map.insert(
            "https://x.com/docs/guide".to_string(),
            "docs/guide/index.md".to_string(),
        );
        let html = r#"<p><a href="https://x.com/docs/guide">g</a> <a href="https://other.com/x">ext</a></p>"#;
        let out = rewrite(html, "docs/intro.md", &map).unwrap();
        assert!(out.contains(r#"href="guide/index.md""#));
        assert!(out.contains(r#"href="https://other.com/x""#));
    }

    #[test]
    fn keeps_image_absolute() {
        let map = BTreeMap::new();
        let html = r#"<p><img src="https://x.com/img/a.png"></p>"#;
        let out = rewrite(html, "docs/intro.md", &map).unwrap();
        assert!(out.contains(r#"src="https://x.com/img/a.png""#));
    }
}
