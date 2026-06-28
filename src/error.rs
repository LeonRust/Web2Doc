//! web2doc 库错误类型（应用层用 `anyhow` 聚合 —— constitution §6 / plan §6.9）。
//!
//! 库路径禁止 `unwrap`/`expect`/`panic!` 处理可恢复错误；统一返回 [`Result`]。

use thiserror::Error;

/// 库内统一错误。各业务模块在其里程碑细化对应变体承载的信息。
#[derive(Debug, Error)]
pub enum Error {
    /// 抓取阶段失败（static/dynamic 引擎）。
    #[error("fetch failed: {0}")]
    Fetch(String),

    /// 正文提取 / 去噪失败。
    #[error("extract failed: {0}")]
    Extract(String),

    /// LLM 规则分析失败（站点级一次）。
    #[error("llm rule analysis failed: {0}")]
    Llm(String),

    /// robots.txt 处理失败。
    #[error("robots handling failed: {0}")]
    Robots(String),

    /// URL 解析 / 规范化失败。
    #[error("invalid url: {0}")]
    Url(String),

    /// 落盘路径越界（逃出 `--out` 目录，constitution §5 安全红线）。
    #[error("path escapes output directory: {0}")]
    PathEscape(String),

    /// I/O 错误。
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// 库内统一 `Result` 别名。
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_converts_via_from() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: Error = io.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn variants_render_messages() {
        assert_eq!(Error::Url("bad".into()).to_string(), "invalid url: bad");
        assert_eq!(
            Error::PathEscape("../x".into()).to_string(),
            "path escapes output directory: ../x"
        );
    }
}
