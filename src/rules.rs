//! 抓取规则集：正文/排除/导航选择器（来自 LLM 或回退默认）。
//!
//! M1 仅使用 [`RuleSet::fallback`]（不调 LLM）；LLM 解析在 M2（plan §6.4）。

use serde::{Deserialize, Serialize};

/// 站点级规则集（plan §5）。各选择器为 CSS 选择器（可用逗号组承载候选链）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    /// 正文容器选择器（候选以逗号分组，匹配任一）。
    pub content_selector: String,
    /// 需移除的噪声选择器。
    pub exclude_selectors: Vec<String>,
    /// 导航链接选择器（用于 discover 收集页面链接）。
    pub nav_link_selector: String,
    /// LLM 判定是否疑似 SPA（用途见 plan §6.4 / T-4）。
    pub looks_like_spa: bool,
}

impl RuleSet {
    /// 回退默认规则（无 key / 网络失败 / 解析失败时使用 —— plan §6.4，覆盖 A6）。
    pub fn fallback() -> Self {
        Self {
            content_selector: "main, article, [role=main], .markdown-body, .content, #content"
                .to_string(),
            exclude_selectors: [
                "nav",
                "aside",
                "header",
                "footer",
                ".sidebar",
                ".toc",
                ".breadcrumb",
                ".pagination",
                ".edit-this-page",
                ".cookie",
                ".search",
            ]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
            nav_link_selector: "nav a, aside a, .sidebar a, .toc a, .menu a".to_string(),
            looks_like_spa: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_has_nonempty_selectors() {
        let r = RuleSet::fallback();
        assert!(r.content_selector.contains("main"));
        assert!(r.content_selector.contains("article"));
        assert!(r.nav_link_selector.contains("nav a"));
        assert!(!r.exclude_selectors.is_empty());
        assert!(r.exclude_selectors.iter().any(|s| s == "footer"));
        assert!(!r.looks_like_spa);
    }

    #[test]
    fn roundtrips_via_serde_json() {
        let r = RuleSet::fallback();
        let json = serde_json::to_string(&r).unwrap();
        let back: RuleSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.content_selector, r.content_selector);
        assert_eq!(back.exclude_selectors, r.exclude_selectors);
    }
}
