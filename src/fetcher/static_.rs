//! 静态抓取引擎（纯 HTTP，reqwest）。适用于 SSR / 静态文档站。

use std::time::Duration;

use url::Url;

use super::{Engine, Fetcher, RenderedPage};
use crate::config::ProxyConfig;
use crate::error::{Error, Result};

const UA: &str = concat!("web2doc/", env!("CARGO_PKG_VERSION"));

/// 基于 `reqwest` 的静态抓取器。
pub struct StaticFetcher {
    client: reqwest::Client,
}

impl StaticFetcher {
    /// 构建静态抓取器（设置 UA 与超时）。`proxy` 为 `None` 时显式直连（屏蔽系统代理隐式探测）。
    pub fn new(proxy: Option<&ProxyConfig>) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .user_agent(UA)
            .timeout(Duration::from_secs(30));
        builder = match proxy {
            Some(p) => builder.proxy(p.reqwest_proxy()?),
            None => builder.no_proxy(),
        };
        let client = builder
            .build()
            .map_err(|e| Error::Fetch(format!("build client: {e}")))?;
        Ok(Self { client })
    }
}

impl Fetcher for StaticFetcher {
    async fn render(&self, url: &Url) -> Result<RenderedPage> {
        let resp = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| Error::Fetch(format!("{url}: {e}")))?;
        let status = resp.status().as_u16();
        let final_url = resp.url().clone();
        let html = resp
            .text()
            .await
            .map_err(|e| Error::Fetch(format!("{url}: read body: {e}")))?;
        Ok(RenderedPage {
            final_url,
            html,
            status,
        })
    }

    fn engine(&self) -> Engine {
        Engine::Static
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_reports_engine() {
        let f = StaticFetcher::new(None).unwrap();
        assert_eq!(f.engine(), Engine::Static);
    }

    #[tokio::test]
    #[ignore = "network: requires internet (example.com)"]
    async fn fetches_real_page() {
        let f = StaticFetcher::new(None).unwrap();
        let page = f
            .render(&Url::parse("https://example.com/").unwrap())
            .await
            .unwrap();
        assert_eq!(page.status, 200);
        assert!(page.html.contains("Example Domain"));
    }
}
