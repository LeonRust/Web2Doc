//! 运行配置：由 CLI + 环境变量 + 配置文件归一后冻结，向下注入（constitution §3）。
//! 业务模块不直接读环境变量。

use std::path::PathBuf;

use serde::Deserialize;
use url::Url;

use crate::cli::{Cli, Mode};
use crate::error::{Error, Result};

/// 默认 LLM 端点（OpenAI 兼容协议）。
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
/// 默认 LLM 模型名。
const DEFAULT_MODEL: &str = "deepseek-v4-flash";

/// 加载 `.env` 文件（如果 CWD 中存在），不会覆盖已设置的环境变量。
fn load_dotenv_file() {
    let _ = dotenvy::dotenv();
}

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

/// 配置文件中的 LLM 段（`~/.config/web2doc/config.toml` > `[llm]`）。
#[derive(Debug, Clone, Default, Deserialize)]
struct LlmFileConfig {
    #[serde(rename = "base_url")]
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
}

/// 配置文件顶层结构。
#[derive(Debug, Clone, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    llm: LlmFileConfig,
}

/// 读取环境变量，过滤空串后返回 `Some(value)`，变量未设置或为空时返回 `None`。
fn env_nonempty(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.is_empty())
}

/// 全局配置目录基路径。
///
/// - **Windows**：`%APPDATA%`（`dirs::config_dir()`）。
/// - **macOS / Linux**：统一遵循 XDG —— `$XDG_CONFIG_HOME`（绝对路径时）否则 `~/.config`。
fn config_base_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        dirs::config_dir()
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
    }
}

/// 从全局配置目录的 `web2doc/config.toml` 加载配置文件（路径见 [`config_base_dir`]）。
/// 文件不存在时静默返回 `None`，解析失败时输出 warning 并忽略。
fn load_file_config() -> Option<FileConfig> {
    let config_dir = config_base_dir()?;
    let path = config_dir.join("web2doc").join("config.toml");
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<FileConfig>(&content) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "配置文件解析失败，已忽略");
                None
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "读取配置文件失败，已忽略");
            None
        }
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
    /// LLM 端点（OpenAI 兼容）。优先级：CLI > env(LLM_BASE_URL) > 配置文件 > 默认。
    pub base_url: String,
    /// LLM 模型名。优先级：CLI > env(LLM_MODEL) > 配置文件 > 默认。
    pub model: String,
    pub max_failure_rate: f64,
    pub bundle: bool,
    pub format: crate::cli::OutputFormat,
    pub ignore_robots: bool,
    pub fresh: bool,
    pub verbose: u8,
    /// LLM API Key。优先级：env(LLM_API_KEY) / .env > 配置文件。
    pub api_key: Option<Secret>,
}

impl Config {
    /// 从 CLI、环境变量、配置文件归一构建配置。
    ///
    /// LLM 三项（base_url / model / api_key）优先级：CLI > 环境变量 > 配置文件 > 默认。
    pub fn from_cli(cli: Cli) -> Result<Self> {
        let start_url =
            Url::parse(&cli.url).map_err(|e| Error::Url(format!("{}: {e}", cli.url)))?;

        load_dotenv_file();
        let file = load_file_config().unwrap_or_default();

        let base_url = cli
            .base_url
            .filter(|v| !v.is_empty())
            .or_else(|| env_nonempty("LLM_BASE_URL"))
            .or(file.llm.base_url)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let model = cli
            .model
            .filter(|v| !v.is_empty())
            .or_else(|| env_nonempty("LLM_MODEL"))
            .or(file.llm.model)
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        // 密钥不接受命令行明文（constitution §5）：仅环境变量 / .env / 配置文件。
        let api_key = env_nonempty("LLM_API_KEY")
            .or(file.llm.api_key)
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
            base_url,
            model,
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
        assert_eq!(c.model, "deepseek-v4-flash");
    }

    #[test]
    fn invalid_url_errors() {
        let err = Config::from_cli(parse(&["web2doc", "not a url"])).unwrap_err();
        assert!(matches!(err, Error::Url(_)));
    }

    #[test]
    fn cli_overrides_llm_defaults() {
        let c = Config::from_cli(parse(&[
            "web2doc",
            "https://x.com/docs/",
            "--base-url",
            "https://api.openai.com/v1",
            "--model",
            "gpt-4o",
        ]))
        .unwrap();
        assert_eq!(c.base_url, "https://api.openai.com/v1");
        assert_eq!(c.model, "gpt-4o");
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
