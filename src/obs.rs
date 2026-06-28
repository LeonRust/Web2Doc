//! tracing / 可观测性初始化（constitution §6 / plan §9）。
//!
//! 结构化字段约定（各模块记录 span/event 时遵循）：
//! `target_url` · `step` · `engine` · `error` · `fallback` · `elapsed_ms` · `llm_calls`。

use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

/// 初始化全局日志订阅器。0=INFO，1=DEBUG，>=2=TRACE；`RUST_LOG` 环境变量可覆盖。
///
/// 幂等：重复调用（如测试）不会 panic（`try_init` 失败被忽略）。
pub fn init(verbose: u8) {
    let level = level_for(verbose);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.to_string()));
    let _ = fmt().with_env_filter(filter).with_target(false).try_init();
}

/// verbose 计数 → 日志级别。
pub fn level_for(verbose: u8) -> Level {
    match verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_maps_to_levels() {
        assert_eq!(level_for(0), Level::INFO);
        assert_eq!(level_for(1), Level::DEBUG);
        assert_eq!(level_for(2), Level::TRACE);
        assert_eq!(level_for(9), Level::TRACE);
    }

    #[test]
    fn init_does_not_panic_when_called_twice() {
        init(0);
        init(2);
    }
}
