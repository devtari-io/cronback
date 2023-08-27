use tracing::log::info;

#[cfg(feature = "admin")]
mod admin;
mod args;
mod client;
mod confirm;
mod runs;
mod triggers;
mod ui;
mod whoami;

use cling::Collected;
pub(crate) use confirm::confirm_or_abort;
use dotenvy::dotenv;

pub use self::args::Cli;

fn init(Collected(verbosity): Collected<clap_verbosity_flag::Verbosity>) {
    // Load .env file. Best effort.
    let maybe_env = dotenv();

    env_logger::Builder::new()
        .filter_level(verbosity.log_level_filter())
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
}
