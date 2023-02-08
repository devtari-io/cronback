mod cli;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use shared::config::{ConfigLoader, Role};
use tokio::task::JoinSet;
use tracing::{info, trace, warn};
use tracing_subscriber::FmtSubscriber;
use valuable::Valuable;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = cli::CliOpts::parse();

    let sub = FmtSubscriber::builder()
        //        .pretty()
        .with_thread_names(true)
        // TODO: Configure logging
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    info!("** {} **", "CronBack.me".magenta());
    trace!(config = opts.config, "Loading configuration");
    let config_loader = Arc::new(ConfigLoader::from_path(&opts.config));

    // Load initial configuration
    let config = config_loader.load()?;

    let mut services = JoinSet::new();
    for role in config.main.roles {
        info!(role = role.as_value(), "Starting service");
        match role {
            Role::Api => services.spawn(api::start_api_server(config_loader.clone())),
            Role::Scheduler => services.spawn(scheduler::start_scheduler(config_loader.clone())),
            Role::Dispatcher => services.spawn(dispatcher::start_dispatcher(config_loader.clone())),
        };
    }

    // Waiting for <C-c> to terminate
    tokio::signal::ctrl_c().await?;
    warn!("Received interrupt signal, terminating servers...");
    services.shutdown().await;

    Ok(())
}
