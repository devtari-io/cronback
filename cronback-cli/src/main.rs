use cling::prelude::*;
use cronback_cli::Cli;
use dotenvy::dotenv;
use tracing::log::info;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ClingFinished<Cli> {
    // Load .env file. Best effort.
    let maybe_env = dotenv();
    let args = Cli::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();
    // We only log after we've initialized the logger with the desired log
    // level.
    match maybe_env {
        | Ok(path) => {
            info!(
                "Loaded environment file from: {}",
                path.to_str().expect("Path not valid UTF-8")
            )
        }
        | Err(e) => info!("Didn't load .env file: {e}"),
    };

    args.into_cling().run().await
}
