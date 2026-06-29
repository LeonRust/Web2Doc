//! 正文提取与去噪（plan §6.5）。
//!
//! M1 取舍：以 `dom_smoothie`（readability）为主力去噪（移除 nav/footer/script 等），
//! 并自动将正文内相对链接绝对化（传入 document_url）；`RuleSet` 的选择器优先策略在 M2 增强。
//! 抓取后二次过滤（T-1）：正文过短 / 无法提取 → [`ExtractOutcome::Excluded`]（含 SPA 空壳）。

use dom_smoothie::Readability;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::rules::RuleSet;

/// 正文文本最小字符数；低于此判为非正文（含 SPA 空壳）。
const MIN_CONTENT_CHARS: usize = 200;

/// 提取出的正文（阶段 A 产物，缓存到 `.cache`）。链接/资源为绝对 URL 字符串。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extracted {
    pub title: String,
    /// 清洗后正文 HTML（尚未做本地化改写，rewrite 在阶段 B）。
    pub content_html: String,
    pub links: Vec<String>,
    pub images: Vec<String>,
    pub embeds: Vec<String>,
}

/// 提取结果：成功正文，或被排除（非正文页 / 空壳）。
#[derive(Debug, Clone)]
pub enum ExtractOutcome {
    Content(Box<Extracted>),
    Excluded(String),
}

/// 从渲染后的 HTML 提取正文。`url` 用于将正文相对链接绝对化。
/// readability 失败或内容过短时回退为 scraper 直接提取正文容器。
pub fn extract(html: &str, url: &Url, _rules: &RuleSet) -> ExtractOutcome {
    let html = expand_tab_content(html);
    let mut readability = match Readability::new(html.as_str(), Some(url.as_str()), None) {
        Ok(r) => r,
        Err(_) => return fallback_extract(&html, url),
    };
    let article = match readability.parse() {
        Ok(a) => a,
        Err(_) => return fallback_extract(&html, url),
    };

    let text_len = article.text_content.trim().chars().count();
    if text_len < MIN_CONTENT_CHARS {
        return fallback_extract(&html, url);
    }

    let content_html = article.content.to_string();
    let (links, images, embeds) = collect_resources(&content_html);

    ExtractOutcome::Content(Box::new(Extracted {
        title: article.title,
        content_html,
        links,
        images,
        embeds,
    }))
}

/// 去掉隐藏属性，让 readability 保留所有 tab / 折叠内容。
fn expand_tab_content(html: &str) -> String {
    html.replace(" hidden=\"\"", "").replace(" hidden>", ">")
}

/// readability 失败 / 内容过短时的回退：直接用 scraper 从 body 取 inner HTML（避免逐候选早停丢失 <pre>）。
fn fallback_extract(html: &str, _url: &Url) -> ExtractOutcome {
    let doc = Html::parse_document(html);
    let title = if let Ok(sel) = Selector::parse("title") {
        doc.select(&sel)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default()
    } else {
        String::new()
    };

    if let Ok(sel) = Selector::parse("body") {
        if let Some(el) = doc.select(&sel).next() {
            let text = el.text().collect::<String>();
            if text.trim().chars().count() >= MIN_CONTENT_CHARS {
                let content_html = el.inner_html();
                let (links, images, embeds) = collect_resources(&content_html);
                return ExtractOutcome::Content(Box::new(Extracted {
                    title,
                    content_html,
                    links,
                    images,
                    embeds,
                }));
            }
        }
    }
    ExtractOutcome::Excluded("fallback: body too short / not found".to_string())
}

/// 从正文 HTML 收集内链 / 图片 / 嵌入资源（绝对 URL 字符串）。
fn collect_resources(html: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let doc = Html::parse_fragment(html);
    let links = select_attr(&doc, "a[href]", "href");
    let images = select_attr(&doc, "img[src]", "src");
    let mut embeds = select_attr(&doc, "video[src]", "src");
    embeds.extend(select_attr(&doc, "iframe[src]", "src"));
    embeds.extend(select_attr(&doc, "source[src]", "src"));
    (links, images, embeds)
}

/// 选择器取属性值（硬编码合法选择器；非法时返回空，不 unwrap — constitution §6）。
fn select_attr(doc: &Html, selector: &str, attr: &str) -> Vec<String> {
    match Selector::parse(selector) {
        Ok(sel) => doc
            .select(&sel)
            .filter_map(|el| el.value().attr(attr))
            .map(str::to_string)
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn collects_resources_from_fragment() {
        let html = r#"<div><p><a href="https://x.com/docs/next">n</a></p>
            <img src="https://x.com/img/a.png"/>
            <iframe src="https://youtube.com/embed/1"></iframe></div>"#;
        let (links, images, embeds) = collect_resources(html);
        assert!(links.iter().any(|s| s.contains("/docs/next")));
        assert!(images.iter().any(|s| s.contains("a.png")));
        assert!(embeds.iter().any(|s| s.contains("youtube.com/embed/1")));
    }

    #[test]
    fn extracts_article_and_excludes_shell() {
        let body = "Rust is a systems programming language focused on safety and performance. \
            It guarantees memory safety without a garbage collector by using its ownership model, \
            where each value has a single owner and the compiler enforces strict borrowing rules. \
            References must always be valid, and the borrow checker rejects programs that could \
            lead to dangling pointers or data races at compile time. Ownership can be transferred \
            through moves, or temporarily shared through immutable and mutable borrows. Because \
            these guarantees are checked statically, Rust programs avoid entire classes of runtime \
            errors that are common in other systems languages, while still producing fast native code.";
        let html = format!(
            "<html><head><title>Ownership</title></head><body>\
             <nav>site menu noise</nav>\
             <article><h1>Ownership</h1><p>{body}</p>\
             <p><img src=\"/img/own.png\"></p></article>\
             <footer>copyright noise</footer></body></html>"
        );
        match extract(
            &html,
            &u("https://doc.rust-lang.org/book/ch04.html"),
            &RuleSet::fallback(),
        ) {
            ExtractOutcome::Content(c) => {
                assert!(!c.content_html.is_empty());
                assert!(c.content_html.contains("ownership") || c.content_html.contains("Rust"));
            }
            ExtractOutcome::Excluded(r) => panic!("expected content, got excluded: {r}"),
        }

        let shell = "<html><body><div id=\"root\"></div></body></html>";
        match extract(shell, &u("https://spa.example.com/"), &RuleSet::fallback()) {
            ExtractOutcome::Excluded(_) => {}
            ExtractOutcome::Content(_) => panic!("shell should be excluded"),
        }
    }

    #[test]
    fn tabpanel_hidden_is_expanded() {
        let html = r#"<div role="tabpanel" hidden><pre>code</pre></div>"#;
        let got = expand_tab_content(html);
        assert!(!got.contains("hidden"), "hidden should be removed: {got}");
        assert!(got.contains(r#"role="tabpanel""#));
    }
}
