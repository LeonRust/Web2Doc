use clap::Parser;

use web2doc::cli::{Cli, Mode};
use web2doc::config::Config;
use web2doc::fetcher::detect::{self, EngineChoice};
use web2doc::fetcher::{AnyFetcher, DynamicFetcher, StaticFetcher};
use web2doc::{obs, pipeline};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    obs::init(cli.verbose);
    let config = Config::from_cli(cli)?;
    tracing::info!(
        target_url = %config.start_url,
        "web2doc {} starting",
        env!("CARGO_PKG_VERSION")
    );

    // 按 --mode + Chrome 检测选择引擎（M4）。
    let detected = detect::detect_chrome(config.chrome_path.as_deref());
    let chrome_missing = detected.is_none();
    let fetcher = match detect::choose(config.mode, detected).map_err(|e| anyhow::anyhow!(e))? {
        EngineChoice::Static => {
            if config.mode == Mode::Auto && chrome_missing {
                tracing::warn!(
                    "未检测到 Chrome，已降级为静态模式（SPA 站点内容可能不全；安装 Chrome 或 --chrome-path 启用动态引擎）"
                );
            }
            AnyFetcher::Static(StaticFetcher::new(config.proxy.as_ref())?)
        }
        EngineChoice::Dynamic(path) => {
            tracing::info!(chrome = %path.display(), "使用动态引擎（Chrome 渲染）");
            AnyFetcher::Dynamic(Box::new(
                DynamicFetcher::launch(&path, config.proxy.as_ref()).await?,
            ))
        }
    };

    let report = pipeline::run(&fetcher, &config).await?;
    report.print();

    std::process::exit(report.exit_code(config.max_failure_rate));
}
