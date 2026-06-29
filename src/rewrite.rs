//! 链接 / 资源改写 + 代码块规范化（唯一改写出口 — plan §6.7 / T-5）。
//!
//! - 内链指向已抓本地页 → 相对化；同站同前缀但未抓取页 → 自动推算本地路径并相对化（B 方案）；外链保持绝对。
//! - 图片命中本地化映射 → 改为相对本地路径（M3）；未命中（未下载/失败）→ 保持绝对。
//! - 代码块：拍平 Prism token `<span>` + `<br>`→`\n`；标题锚点链接去除。

use std::collections::BTreeMap;

use lol_html::html_content::{ContentType, Element};
use lol_html::{element, rewrite_str, RewriteStrSettings};
use url::Url;

use crate::error::{Error, Result};
use crate::urlx;

/// 改写正文 HTML。
/// - `page_map`：`dedup_key(绝对 URL)` → 镜像 `rel_path`（已抓取页）。
/// - `asset_map`：图片绝对 URL → 相对当前页的本地路径（图片本地化）。
/// - `host` / `prefixes`：站内判定 → 未抓取页自动推算 `map_single_path`（B 方案）。
pub fn rewrite(
    content_html: &str,
    current_rel: &str,
    page_map: &BTreeMap<String, String>,
    asset_map: &BTreeMap<String, String>,
    host: &str,
    prefixes: &[String],
) -> Result<String> {
    let settings = RewriteStrSettings::new()
        .append_element_content_handler(element!("a[href]", |el: &mut Element| {
            if let Some(href) = el.get_attribute("href") {
                if let Some(rel) = remap_internal_link(&href, current_rel, page_map, host, prefixes)
                {
                    let _ = el.set_attribute("href", &rel);
                }
            }
            Ok(())
        }))
        .append_element_content_handler(element!("img[src]", |el: &mut Element| {
            if let Some(src) = el.get_attribute("src") {
                if let Some(local) = asset_map.get(&src) {
                    let _ = el.set_attribute("src", local);
                }
            }
            Ok(())
        }))
        .append_element_content_handler(element!("pre span", |el: &mut Element| {
            el.remove_and_keep_content();
            Ok(())
        }))
        .append_element_content_handler(element!("pre br", |el: &mut Element| {
            el.replace("\n", ContentType::Text);
            Ok(())
        }))
        .append_element_content_handler(element!(
            "h1 a, h2 a, h3 a, h4 a, h5 a, h6 a",
            |el: &mut Element| {
                el.remove();
                Ok(())
            }
        ));
    rewrite_str(content_html, settings).map_err(|e| Error::Extract(format!("rewrite: {e}")))
}

/// 若 `href` 指向站内文档页，返回相对当前页的本地路径；否则 `None`（外站保留绝对）。
fn remap_internal_link(
    href: &str,
    current_rel: &str,
    page_map: &BTreeMap<String, String>,
    host: &str,
    prefixes: &[String],
) -> Option<String> {
    let abs = Url::parse(href).ok()?;
    let key = urlx::dedup_key(&abs);
    // 1) 已抓取 → page_map 命中
    if let Some(rel) = page_map.get(&key) {
        return Some(urlx::relative_path(current_rel, rel));
    }
    // 2) 站内同前缀但未抓取 → 推算单 URL 路径并相对化（B 方案）
    if urlx::in_prefix(&abs, host, prefixes) {
        let inferred = urlx::map_single_path(&abs);
        return Some(urlx::relative_path(current_rel, &inferred));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> BTreeMap<String, String> {
        BTreeMap::new()
    }
    const HOST: &str = "x.com";
    const PREFIX: &[String] = &[];

    #[test]
    fn relativizes_internal_link_keeps_external() {
        let mut map = BTreeMap::new();
        map.insert(
            "https://x.com/docs/guide".to_string(),
            "docs/guide/index.md".to_string(),
        );
        let html = r#"<p><a href="https://x.com/docs/guide">g</a> <a href="https://other.com/x">ext</a></p>"#;
        let out = rewrite(html, "docs/intro.md", &map, &empty(), HOST, PREFIX).unwrap();
        assert!(out.contains(r#"href="guide/index.md""#));
        assert!(out.contains(r#"href="https://other.com/x""#));
    }

    #[test]
    fn localizes_image_keeps_missing_absolute() {
        let mut assets = BTreeMap::new();
        assets.insert(
            "https://x.com/img/a.png".to_string(),
            "../assets/aa.png".to_string(),
        );
        let html =
            r#"<p><img src="https://x.com/img/a.png"><img src="https://x.com/img/b.png"></p>"#;
        let out = rewrite(html, "docs/intro.md", &empty(), &assets, HOST, PREFIX).unwrap();
        assert!(
            out.contains(r#"src="../assets/aa.png""#),
            "localized: {out}"
        );
        assert!(
            out.contains(r#"src="https://x.com/img/b.png""#),
            "missing stays absolute: {out}"
        );
    }

    #[test]
    fn infers_path_for_unvisited_same_prefix_link() {
        // 站内但不在 page_map → 自动推算本地路径（B 方案）
        let html = r#"<a href="https://x.com/docs/unvisited">link</a>"#;
        let out = rewrite(
            html,
            "docs/index.md",
            &empty(),
            &empty(),
            "x.com",
            &["/docs/".to_string()],
        )
        .unwrap();
        assert!(out.contains(r#"href="unvisited.md""#), "got: {out}");
    }

    #[test]
    fn code_block_br_becomes_newline() {
        let out = rewrite(
            "<pre><code>a<br>b<br>c</code></pre>",
            "a.md",
            &empty(),
            &empty(),
            HOST,
            PREFIX,
        )
        .unwrap();
        assert!(!out.contains("<br"), "br should be gone: {out}");
        assert!(out.contains("a\nb\nc"), "got: {out}");
    }

    #[test]
    fn code_block_flattens_nested_spans_and_newlines() {
        let html = "<pre><code><span><span>import</span><span> json</span><br></span>\
                    <span><span>x</span><span> = </span><span>1</span><br></span></code></pre>";
        let out = rewrite(html, "a.md", &empty(), &empty(), HOST, PREFIX).unwrap();
        assert!(
            !out.contains("<span"),
            "spans in pre should be flattened: {out}"
        );
        assert!(!out.contains("<br"), "br should be gone: {out}");
        assert!(out.contains("import json\nx = 1"), "got: {out}");
    }

    #[test]
    fn br_outside_pre_is_untouched() {
        let out = rewrite("<p>x<br>y</p>", "a.md", &empty(), &empty(), HOST, PREFIX).unwrap();
        assert!(out.contains("<br"), "non-code <br> should remain: {out}");
    }

    #[test]
    fn strips_heading_anchor_links() {
        let html = r#"<h2 id="x">Title<a href='#x' aria-label='link to Title'>#</a></h2>"#;
        let out = rewrite(html, "a.md", &empty(), &empty(), HOST, PREFIX).unwrap();
        assert!(!out.contains("<a"), "heading anchor removed: {out}");
        assert!(out.contains("Title"), "heading text preserved: {out}");
    }
}
