use anyhow::Result;
use clap::Parser;
use cronback_cli::{run_cli, Cli};
use dotenvy::dotenv;
use tracing::log::info;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = Cli::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();
    // Load .env file. Best effort.
    if let Err(e) = dotenv() {
        info!("Didn't load .env file: {e}");
    };
    run_cli(args).await
}
