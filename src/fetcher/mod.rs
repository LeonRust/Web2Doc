//! 抓取引擎：trait 抽象 + 实现（constitution §3：上层只依赖 trait）。

pub mod detect;
mod dynamic;
mod static_;

pub use dynamic::DynamicFetcher;
pub use static_::StaticFetcher;

use url::Url;

use crate::error::Result;

/// 抓取引擎类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Static,
    Dynamic,
}

/// 单页渲染结果。
#[derive(Debug, Clone)]
pub struct RenderedPage {
    /// 跟随重定向后的最终 URL。
    pub final_url: Url,
    /// 页面 HTML（静态引擎为原始响应；动态引擎为渲染后 DOM）。
    pub html: String,
    /// HTTP 状态码。
    pub status: u16,
}

/// 抓取引擎抽象。
///
/// 采用原生 async fn in trait；经具体类型 / [`AnyFetcher`] 枚举分发（非 `dyn`），
/// future 的 `Send` 性由各实现保证。
#[allow(async_fn_in_trait)]
pub trait Fetcher {
    /// 抓取并（动态引擎时）渲染单页。
    async fn render(&self, url: &Url) -> Result<RenderedPage>;

    /// 引擎类型（用于日志/报告）。
    fn engine(&self) -> Engine;
}

/// 运行时引擎分发：main 据 `--mode` 与 Chrome 检测选择静态或动态。
pub enum AnyFetcher {
    Static(StaticFetcher),
    Dynamic(Box<DynamicFetcher>),
}

impl Fetcher for AnyFetcher {
    async fn render(&self, url: &Url) -> Result<RenderedPage> {
        match self {
            AnyFetcher::Static(f) => f.render(url).await,
            AnyFetcher::Dynamic(f) => f.render(url).await,
        }
    }

    fn engine(&self) -> Engine {
        match self {
            AnyFetcher::Static(f) => f.engine(),
            AnyFetcher::Dynamic(f) => f.engine(),
        }
    }
}
