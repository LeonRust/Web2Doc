//! HTML → Markdown 转换（htmd）— plan §6.5 / M1.12。保留标题、代码块、表格、链接等结构。

use crate::error::{Error, Result};

/// 将（已改写的）正文 HTML 转为 Markdown。
pub fn to_markdown(html: &str) -> Result<String> {
    htmd::convert(html).map_err(|e| Error::Extract(format!("html→md: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_heading() {
        assert_eq!(to_markdown("<h1>Title</h1>").unwrap().trim(), "# Title");
    }

    #[test]
    fn converts_code_block() {
        let md = to_markdown("<pre><code>let x = 1;</code></pre>").unwrap();
        assert!(md.contains("```"));
        assert!(md.contains("let x = 1;"));
    }

    #[test]
    fn converts_table() {
        let html = "<table><thead><tr><th>A</th><th>B</th></tr></thead>\
                    <tbody><tr><td>1</td><td>2</td></tr></tbody></table>";
        let md = to_markdown(html).unwrap();
        assert!(md.contains('|'), "expected GFM table pipes, got: {md}");
    }

    #[test]
    fn keeps_relative_link() {
        let md = to_markdown(r#"<a href="guide/b.md">Guide</a>"#).unwrap();
        assert!(md.contains("[Guide](guide/b.md)"), "got: {md}");
    }
}
