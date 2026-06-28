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

    /// LLM 端点（OpenAI 兼容）。
    #[arg(long, default_value = "https://api.deepseek.com")]
    pub base_url: String,

    /// LLM 模型名。
    #[arg(long, default_value = "deepseek-chat")]
    pub model: String,

    /// 失败率阈值（超过判整次失败）。
    #[arg(long, default_value_t = 0.20)]
    pub max_failure_rate: f64,

    /// 额外输出全文合并文件。
    #[arg(long)]
    pub bundle: bool,

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
        assert_eq!(cli.base_url, "https://api.deepseek.com");
        assert_eq!(cli.model, "deepseek-chat");
        assert!(!cli.bundle);
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
}
