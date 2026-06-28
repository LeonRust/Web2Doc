use clap::Parser;

use web2doc::cli::Cli;
use web2doc::config::Config;
use web2doc::fetcher::StaticFetcher;
use web2doc::{obs, pipeline};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::from_cli(cli)?;
    obs::init(config.verbose);
    tracing::info!(
        target_url = %config.start_url,
        "web2doc {} starting",
        env!("CARGO_PKG_VERSION")
    );

    // M1：静态引擎（动态引擎在 M4 按 --mode 选择）。
    let fetcher = StaticFetcher::new()?;
    let report = pipeline::run(&fetcher, &config).await?;
    report.print();

    std::process::exit(report.exit_code(config.max_failure_rate));
}
