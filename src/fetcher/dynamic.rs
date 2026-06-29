//! 动态抓取引擎（chromiumoxide 驱动 headless Chrome 渲染 SPA）— M4.1 / C1。

use std::path::Path;
use std::time::Duration;

use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use tokio::task::JoinHandle;
use url::Url;

use super::{Engine, Fetcher, RenderedPage};
use crate::error::{Error, Result};

/// 基于 chromiumoxide 的动态抓取器：渲染后取 DOM。
pub struct DynamicFetcher {
    browser: Browser,
    _handler: JoinHandle<()>,
    timeout: Duration,
}

impl DynamicFetcher {
    /// 启动 headless Chrome（指定可执行路径）。
    pub async fn launch(chrome_path: &Path) -> Result<Self> {
        // 唯一 user-data-dir，避免默认固定 profile 的 SingletonLock 冲突（多次运行 — constitution §8）。
        let user_data = std::env::temp_dir().join(format!("web2doc-chrome-{}", std::process::id()));
        let config = BrowserConfig::builder()
            .chrome_executable(chrome_path)
            .user_data_dir(&user_data)
            .no_sandbox()
            .build()
            .map_err(|e| Error::Fetch(format!("browser config: {e}")))?;
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| Error::Fetch(format!("launch chrome: {e}")))?;
        // 驱动 CDP 事件循环（浏览器存活期间）。
        let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });
        Ok(Self {
            browser,
            _handler: handler_task,
            timeout: Duration::from_secs(30),
        })
    }

    async fn render_inner(&self, url: &Url) -> Result<String> {
        let page = self
            .browser
            .new_page(url.as_str())
            .await
            .map_err(|e| Error::Fetch(format!("{url}: new_page: {e}")))?;
        let _ = page.wait_for_navigation().await;
        // SPA 渲染缓冲：异步加载内容需要额外时间完成。
        tokio::time::sleep(Duration::from_millis(200)).await;
        let html = page
            .content()
            .await
            .map_err(|e| Error::Fetch(format!("{url}: content: {e}")))?;
        let _ = page.close().await;
        Ok(html)
    }
}

impl Fetcher for DynamicFetcher {
    async fn render(&self, url: &Url) -> Result<RenderedPage> {
        let html = tokio::time::timeout(self.timeout, self.render_inner(url))
            .await
            .map_err(|_| Error::Fetch(format!("{url}: render timeout")))??;
        Ok(RenderedPage {
            final_url: url.clone(),
            html,
            status: 200,
        })
    }

    fn engine(&self) -> Engine {
        Engine::Dynamic
    }
}
