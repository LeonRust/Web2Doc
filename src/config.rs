//! 运行配置：由 CLI + 环境变量归一后冻结，向下注入（constitution §3）。
//! 业务模块不直接读环境变量。

use std::path::PathBuf;

use url::Url;

use crate::cli::{Cli, Mode};
use crate::error::{Error, Result};

/// 敏感字符串包装：`Debug` 输出脱敏，防止密钥进入日志 / 产物（constitution §5）。
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    /// 暴露明文（仅在向 LLM 端点发请求时调用，不得写入日志）。
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"***\"")
    }
}

/// 冻结后的运行配置。
#[derive(Debug, Clone)]
pub struct Config {
    pub start_url: Url,
    pub out_dir: PathBuf,
    /// 显式前缀（`None` = 默认取 `start_url` 路径目录，于 discover 阶段用 `urlx` 推导）。
    pub prefix: Option<String>,
    pub include_prefixes: Vec<String>,
    pub max_pages: usize,
    pub concurrency: usize,
    pub delay_ms: u64,
    pub mode: Mode,
    pub chrome_path: Option<PathBuf>,
    pub base_url: String,
    pub model: String,
    pub max_failure_rate: f64,
    pub bundle: bool,
    pub format: crate::cli::OutputFormat,
    pub ignore_robots: bool,
    pub fresh: bool,
    pub verbose: u8,
    /// LLM API Key，仅来自环境变量（`OPENAI_API_KEY`，兼容 `DEEPSEEK_API_KEY`）。
    pub api_key: Option<Secret>,
}

impl Config {
    /// 从 CLI 与环境变量归一构建配置。
    pub fn from_cli(cli: Cli) -> Result<Self> {
        let start_url =
            Url::parse(&cli.url).map_err(|e| Error::Url(format!("{}: {e}", cli.url)))?;

        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
            .ok()
            .filter(|k| !k.is_empty())
            .map(Secret);

        Ok(Config {
            start_url,
            out_dir: cli.out,
            prefix: cli.prefix,
            include_prefixes: cli.include_prefix,
            max_pages: cli.max_pages,
            concurrency: cli.concurrency,
            delay_ms: cli.delay_ms,
            mode: cli.mode,
            chrome_path: cli.chrome_path,
            base_url: cli.base_url,
            model: cli.model,
            max_failure_rate: cli.max_failure_rate,
            bundle: cli.bundle,
            format: cli.format,
            ignore_robots: cli.ignore_robots,
            fresh: cli.fresh,
            verbose: cli.verbose,
            api_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).unwrap()
    }

    #[test]
    fn builds_from_cli_with_valid_url() {
        let c = Config::from_cli(parse(&["web2doc", "https://example.com/docs/intro"])).unwrap();
        assert_eq!(c.start_url.host_str(), Some("example.com"));
        assert_eq!(c.max_pages, 500);
        assert_eq!(c.base_url, "https://api.deepseek.com");
    }

    #[test]
    fn invalid_url_errors() {
        let err = Config::from_cli(parse(&["web2doc", "not a url"])).unwrap_err();
        assert!(matches!(err, Error::Url(_)));
    }

    #[test]
    fn secret_is_redacted_in_debug() {
        let s = Secret("super-secret-key".to_string());
        let rendered = format!("{s:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains("***"));
    }

    #[test]
    fn config_debug_does_not_leak_key() {
        let mut c = Config::from_cli(parse(&["web2doc", "https://x.com/docs/"])).unwrap();
        c.api_key = Some(Secret("leak-me".to_string()));
        let rendered = format!("{c:?}");
        assert!(!rendered.contains("leak-me"));
    }
}
