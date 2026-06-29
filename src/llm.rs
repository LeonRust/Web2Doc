//! LLM 规则分析（OpenAI 兼容 Chat Completions，站点级一次 — plan §6.4 / M2 / S7）。
//!
//! 完整降级链：调用失败 / 非 JSON → 回退默认；缺字段填默认；非法 CSS 剔除；
//! `content_selector` 首页 0 命中 → 回退候选链。全程仅 1 次调用（不随页数增长，A5）。

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::rules::RuleSet;

const SYSTEM_PROMPT: &str = "You analyze a documentation website's homepage HTML and return CSS \
selectors for crawling it. Return ONLY a JSON object with keys: content_selector (string, CSS \
selector for the main article/content container), exclude_selectors (array of strings, CSS \
selectors for nav/header/footer/sidebar/ads to remove), nav_link_selector (string, CSS selector \
matching navigation links to other doc pages), looks_like_spa (boolean, whether the page is a \
client-rendered SPA shell with little server content). Comma-separated selectors are allowed.";

const MAX_INPUT_CHARS: usize = 12_000;

/// OpenAI 兼容 Chat Completions 客户端。
pub struct LlmClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl LlmClient {
    pub fn new(http: reqwest::Client, base_url: &str, model: &str, api_key: &str) -> Self {
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// 分析首页 → 经校验的 `RuleSet`（含完整降级链）。仅 1 次 LLM 调用。
    pub async fn analyze(&self, home_html: &str) -> RuleSet {
        match self.call(&skeleton(home_html)).await {
            Ok(content) => {
                let rules = validate_ruleset(parse_ruleset(&content), home_html);
                tracing::info!(
                    content_selector = %rules.content_selector,
                    exclude_count = rules.exclude_selectors.len(),
                    looks_like_spa = rules.looks_like_spa,
                    "LLM 规则分析成功"
                );
                rules
            }
            Err(e) => {
                tracing::warn!(error = %e, "LLM 调用失败，回退默认规则");
                RuleSet::fallback()
            }
        }
    }

    async fn call(&self, user_content: &str) -> Result<String> {
        let req = ChatRequest {
            model: &self.model,
            messages: vec![
                Message {
                    role: "system",
                    content: SYSTEM_PROMPT,
                },
                Message {
                    role: "user",
                    content: user_content,
                },
            ],
            response_format: ResponseFormat {
                kind: "json_object",
            },
            temperature: 0.0,
        };
        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| Error::Llm(format!("request: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::Llm(format!("status {}", resp.status())));
        }
        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| Error::Llm(format!("decode: {e}")))?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| Error::Llm("empty choices".to_string()))
    }
}

/// 粗略判断首页是否疑似 SPA 空壳（可见词数极少）。
pub fn looks_empty(html: &str) -> bool {
    let doc = Html::parse_document(html);
    let text: String = doc.root_element().text().collect::<Vec<_>>().join(" ");
    text.split_whitespace().count() < 30
}

/// 首页结构骨架（截断控 token）。M2 简化为按字符截断。
fn skeleton(html: &str) -> String {
    if html.len() <= MAX_INPUT_CHARS {
        html.to_string()
    } else {
        html.chars().take(MAX_INPUT_CHARS).collect()
    }
}

/// 解析 LLM 返回 JSON → RuleSet（未知字段忽略、缺字段填默认；非法 JSON → 回退）。
fn parse_ruleset(json: &str) -> RuleSet {
    #[derive(Deserialize, Default)]
    struct Partial {
        content_selector: Option<String>,
        exclude_selectors: Option<Vec<String>>,
        nav_link_selector: Option<String>,
        looks_like_spa: Option<bool>,
    }
    let fb = RuleSet::fallback();
    match serde_json::from_str::<Partial>(json) {
        Ok(p) => RuleSet {
            content_selector: p
                .content_selector
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(fb.content_selector),
            exclude_selectors: p
                .exclude_selectors
                .filter(|v| !v.is_empty())
                .unwrap_or(fb.exclude_selectors),
            nav_link_selector: p
                .nav_link_selector
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(fb.nav_link_selector),
            looks_like_spa: p.looks_like_spa.unwrap_or(false),
        },
        Err(_) => fb,
    }
}

/// 校验并修复：剔除非法 CSS；`content_selector` 首页 0 命中 / 非法 → 回退默认。
fn validate_ruleset(rules: RuleSet, home_html: &str) -> RuleSet {
    let fb = RuleSet::fallback();

    let exclude_selectors: Vec<String> = rules
        .exclude_selectors
        .into_iter()
        .filter(|s| Selector::parse(s).is_ok())
        .collect();

    let nav_ok = Selector::parse(&rules.nav_link_selector).is_ok();
    let nav_link_selector = if nav_ok {
        rules.nav_link_selector
    } else {
        fb.nav_link_selector.clone()
    };

    let doc = Html::parse_document(home_html);
    let content_hits = Selector::parse(&rules.content_selector)
        .ok()
        .map(|sel| doc.select(&sel).next().is_some())
        .unwrap_or(false);
    let content_selector = if content_hits {
        rules.content_selector
    } else {
        fb.content_selector
    };

    RuleSet {
        content_selector,
        exclude_selectors,
        nav_link_selector,
        looks_like_spa: rules.looks_like_spa,
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    response_format: ResponseFormat,
    temperature: f64,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: RespMessage,
}

#[derive(Deserialize)]
struct RespMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_json() {
        let json = r#"{"content_selector":"main","exclude_selectors":["nav",".x"],"nav_link_selector":"a","looks_like_spa":true}"#;
        let r = parse_ruleset(json);
        assert_eq!(r.content_selector, "main");
        assert_eq!(r.exclude_selectors, vec!["nav", ".x"]);
        assert!(r.looks_like_spa);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let r = parse_ruleset(r#"{"content_selector":"article"}"#);
        assert_eq!(r.content_selector, "article");
        assert!(!r.exclude_selectors.is_empty());
        assert!(!r.nav_link_selector.is_empty());
        assert!(!r.looks_like_spa);
    }

    #[test]
    fn invalid_json_falls_back() {
        let r = parse_ruleset("not json at all");
        assert_eq!(r.content_selector, RuleSet::fallback().content_selector);
    }

    #[test]
    fn validate_drops_invalid_css_and_keeps_hit() {
        let home = "<html><body><main>hello</main></body></html>";
        let rules = RuleSet {
            content_selector: "main".into(),
            exclude_selectors: vec!["nav".into(), ">>>bad".into()],
            nav_link_selector: "a".into(),
            looks_like_spa: false,
        };
        let v = validate_ruleset(rules, home);
        assert_eq!(v.content_selector, "main");
        assert_eq!(v.exclude_selectors, vec!["nav"]);
    }

    #[test]
    fn validate_falls_back_when_content_selector_misses() {
        let home = "<html><body><div>x</div></body></html>";
        let rules = RuleSet {
            content_selector: "article.nope".into(),
            exclude_selectors: vec![],
            nav_link_selector: "a".into(),
            looks_like_spa: false,
        };
        let v = validate_ruleset(rules, home);
        assert_eq!(v.content_selector, RuleSet::fallback().content_selector);
    }

    #[test]
    fn empty_shell_detected() {
        assert!(looks_empty(
            "<html><body><div id=\"root\"></div></body></html>"
        ));
        assert!(!looks_empty(&format!("<p>{}</p>", "word ".repeat(50))));
    }
}
