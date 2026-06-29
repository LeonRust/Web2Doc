//! HTML → Markdown 转换（htmd）— plan §6.5 / M1.12。保留标题、代码块、表格、链接等结构。

use crate::error::{Error, Result};

/// 将（已改写的）正文 HTML 转为 Markdown。
pub fn to_markdown(html: &str) -> Result<String> {
    htmd::convert(html).map_err(|e| Error::Extract(format!("html→md: {e}")))
}

/// 规范化 `<table>`：把第一个无 `<th>` 的 `<tr>` 中的 `<td>` 改成 `<th>`，
/// 让 htmd 能生成 GFM 表头分隔符（Docusaurus 定价表等场景）。
pub fn fix_tables(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(tbl_start) = rest.find("<table") {
        // 扩展范围：<b><table>…</table></b> 模式（Docusaurus 定价表被粗体包裹）。
        let start = if tbl_start >= 3 && rest.as_bytes()[tbl_start - 3] == b'<' {
            let pre = &rest[tbl_start - 3..tbl_start];
            if pre == "<b>" || pre == "<st" {
                tbl_start - 3
            } else {
                tbl_start
            }
        } else {
            tbl_start
        };
        out.push_str(&rest[..start]);
        rest = &rest[start..];

        let tbl_end = match rest.find("</table>") {
            Some(i) => i + 8,
            None => rest.len(),
        };
        let end = if rest[tbl_end..].starts_with("</b>") {
            tbl_end + 4
        } else if rest[tbl_end..].starts_with("</strong>") {
            tbl_end + 9
        } else {
            tbl_end
        };

        let fixed = fix_one_table(&rest[..end]);
        out.push_str(&fixed);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn fix_one_table(tbl: &str) -> String {
    let tbl = unwrap_table_wrapper(tbl);
    if let Some(tr_start) = tbl.find("<tr") {
        if let Some(tr_end) = tbl[tr_start..].find("</tr>") {
            let tr_end_abs = tr_start + tr_end + 5;
            let first_tr = &tbl[tr_start..tr_end_abs];
            if !first_tr.contains("<th") {
                let fixed_tr = first_tr.replace("<td", "<th").replace("</td>", "</th>");
                return format!("{}{}{}", &tbl[..tr_start], fixed_tr, &tbl[tr_end_abs..]);
            }
        }
    }
    tbl.to_string()
}

/// 剥离 <b>/<strong> 包裹 <table> 的标签（Docusaurus 定价表被粗体包裹 → htmd 输出 ** 破坏表格）。
fn unwrap_table_wrapper(html: &str) -> String {
    for tag in ["b", "strong"] {
        let prefix = format!("<{tag}><table");
        let suffix = format!("</table></{tag}>");
        if html.starts_with(&prefix) && html.ends_with(&suffix) {
            return html[prefix.len() - 6..html.len() - suffix.len() + 8].to_string();
        }
    }
    html.to_string()
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

    #[test]
    fn converts_multiline_code_block() {
        let md = to_markdown("<pre><code>a\nb\nc</code></pre>").unwrap();
        assert!(
            md.contains("a\nb\nc"),
            "htmd should preserve newlines, got: {md}"
        );
    }

    #[test]
    fn fix_table_td_to_th() {
        let tbl = "<table><tbody><tr><td>Col1</td><td>Col2</td></tr><tr><td>a</td><td>b</td></tr></tbody></table>";
        let got = fix_one_table(tbl);
        assert!(got.contains("<th>Col1</th>"), "got: {got}");
        assert!(got.contains("<th>Col2</th>"));
        assert!(!got.contains("<td>Col1</td>"));
        assert!(got.contains("<td>a</td>")); // 第二行不变
    }

    #[test]
    fn fix_table_skips_existing_th() {
        let tbl = "<table><tr><th>A</th></tr></table>";
        assert_eq!(fix_one_table(tbl), tbl);
    }

    #[test]
    fn unwrap_b_wrapped_table() {
        let got = fix_one_table("<b><table><tr><td>X</td></tr></table></b>");
        assert!(!got.contains("<b>"), "should strip <b>: {got}");
        assert!(got.contains("<th>X</th>"), "and td→th: {got}");
    }

    #[test]
    fn fix_tables_handles_multiple() {
        let html =
            "<p>a</p><table><tr><td>X</td></tr></table><p>b</p><table><tr><td>Y</td></tr></table>";
        let got = fix_tables(html);
        assert!(got.contains("<th>X</th>"));
        assert!(got.contains("<th>Y</th>"));
        assert!(got.contains("<p>b</p>"));
    }
}
