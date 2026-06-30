//! 命令行参数定义（clap derive，对应 plan §4）。

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// 抓取引擎模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    /// 自动：检测到 Chrome 用动态引擎，否则降级静态并告警。
    #[default]
    Auto,
    /// 仅静态抓取（纯 HTTP）。
    Static,
    /// 强制动态引擎（无 Chrome 则报错退出）。
    Dynamic,
}

/// 输出格式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Markdown（默认，适合大多数文档站）。
    #[default]
    Md,
    /// HTML（保真度高：表格/隐藏 tab/代码块结构完整保留，适合复杂 API 文档）。
    Html,
}

/// web2doc：抓取在线文档站为本地结构化 Markdown。
#[derive(Debug, Parser)]
#[command(name = "web2doc", version, about)]
pub struct Cli {
    /// 文档站首页 URL。
    pub url: String,

    /// 产物输出目录。
    #[arg(long, default_value = "./web2doc-out")]
    pub out: PathBuf,

    /// 覆盖抓取前缀（默认取 URL 路径目录）。
    #[arg(long)]
    pub prefix: Option<String>,

    /// 追加允许前缀（可多次）。
    #[arg(long = "include-prefix")]
    pub include_prefix: Vec<String>,

    /// 最大页数上限。
    #[arg(long, default_value_t = 500)]
    pub max_pages: usize,

    /// 并发数。
    #[arg(long, default_value_t = 4)]
    pub concurrency: usize,

    /// 请求间隔（毫秒）。
    #[arg(long, default_value_t = 500)]
    pub delay_ms: u64,

    /// 抓取引擎模式。
    #[arg(long, value_enum, default_value_t = Mode::Auto)]
    pub mode: Mode,

    /// 指定 Chrome 可执行文件路径。
    #[arg(long)]
    pub chrome_path: Option<PathBuf>,

    /// LLM 端点（OpenAI 兼容）。优先级：CLI > env(LLM_BASE_URL) / .env > 配置文件 > 默认 https://api.deepseek.com。
    #[arg(long)]
    pub base_url: Option<String>,

    /// LLM 模型名。优先级：CLI > env(LLM_MODEL) / .env > 配置文件 > 默认 deepseek-v4-flash。
    #[arg(long)]
    pub model: Option<String>,

    /// 出站代理 URL（http/https/socks5）。优先级：CLI > env(ALL_PROXY/HTTPS_PROXY/HTTP_PROXY) / .env > 配置文件。
    #[arg(long)]
    pub proxy: Option<String>,

    /// 代理绕过列表（逗号分隔，如 `localhost,127.0.0.1,*.internal`）。亦可经 env(NO_PROXY) / 配置文件。
    #[arg(long = "no-proxy")]
    pub no_proxy: Option<String>,

    /// 失败率阈值（超过判整次失败）。
    #[arg(long, default_value_t = 0.20)]
    pub max_failure_rate: f64,

    /// 额外输出全文合并文件。
    #[arg(long)]
    pub bundle: bool,

    /// 输出格式：md（Markdown，默认）或 html。
    #[arg(long, value_enum, default_value = "md")]
    pub format: OutputFormat,

    /// 忽略 robots.txt（默认尊重）。
    #[arg(long)]
    pub ignore_robots: bool,

    /// 忽略既有 manifest，重新抓取（默认自动续传）。
    #[arg(long)]
    pub fresh: bool,

    /// 日志详细度（-v / -vv）。
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_defaults() {
        let cli = Cli::try_parse_from(["web2doc", "https://example.com/docs/"]).unwrap();
        assert_eq!(cli.url, "https://example.com/docs/");
        assert_eq!(cli.max_pages, 500);
        assert_eq!(cli.concurrency, 4);
        assert_eq!(cli.delay_ms, 500);
        assert_eq!(cli.mode, Mode::Auto);
        assert_eq!(cli.base_url, None);
        assert_eq!(cli.model, None);
        assert_eq!(cli.proxy, None);
        assert_eq!(cli.no_proxy, None);
        assert!(!cli.bundle);
        assert_eq!(cli.format, OutputFormat::Md);
        assert!(!cli.ignore_robots);
        assert!(!cli.fresh);
    }

    #[test]
    fn missing_url_is_error() {
        assert!(Cli::try_parse_from(["web2doc"]).is_err());
    }

    #[test]
    fn include_prefix_is_repeatable() {
        let cli = Cli::try_parse_from([
            "web2doc",
            "https://x.com/docs/",
            "--include-prefix",
            "/api/",
            "--include-prefix",
            "/guide/",
        ])
        .unwrap();
        assert_eq!(cli.include_prefix, vec!["/api/", "/guide/"]);
    }

    #[test]
    fn verbose_counts() {
        let cli = Cli::try_parse_from(["web2doc", "https://x.com/", "-vv"]).unwrap();
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn llm_flags_parse_into_some() {
        let cli = Cli::try_parse_from([
            "web2doc",
            "https://x.com/docs/",
            "--base-url",
            "https://api.openai.com/v1",
            "--model",
            "gpt-4o",
        ])
        .unwrap();
        assert_eq!(cli.base_url.as_deref(), Some("https://api.openai.com/v1"));
        assert_eq!(cli.model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn proxy_flags_parse_into_some() {
        let cli = Cli::try_parse_from([
            "web2doc",
            "https://x.com/docs/",
            "--proxy",
            "socks5://127.0.0.1:1080",
            "--no-proxy",
            "localhost,127.0.0.1",
        ])
        .unwrap();
        assert_eq!(cli.proxy.as_deref(), Some("socks5://127.0.0.1:1080"));
        assert_eq!(cli.no_proxy.as_deref(), Some("localhost,127.0.0.1"));
    }
}
