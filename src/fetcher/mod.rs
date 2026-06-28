//! 抓取引擎：trait 抽象 + 实现（constitution §3：上层只依赖 trait）。

mod static_;

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

/// 抓取引擎抽象。M1 仅 [`StaticFetcher`]；动态引擎在 M4。
///
/// 采用原生 async fn in trait；M1 经具体类型 / 枚举分发（非 `dyn`），
/// future 的 `Send` 性由各实现保证（reqwest future 为 Send）。
#[allow(async_fn_in_trait)]
pub trait Fetcher {
    /// 抓取并（动态引擎时）渲染单页。
    async fn render(&self, url: &Url) -> Result<RenderedPage>;

    /// 引擎类型（用于日志/报告）。
    fn engine(&self) -> Engine;
}
