mod cli;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use shared::config::{CoreConfig, Role};
use tokio::task::JoinSet;
use tracing::{info, trace};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = cli::CliOpts::parse();

    let sub = FmtSubscriber::builder()
        .pretty()
        .with_thread_names(true)
        // TODO: Configure logging
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    info!("** {} **", "CronBack.me".magenta());
    trace!(config = opts.config, "Loading configuration");
    let config = CoreConfig::from_path(&opts.config)?;

    let mut services = JoinSet::new();
    for role in &config.roles {
        info!("Booting role {:?}", role);
        match role {
            Role::Api => services.spawn(api::start_api_server(config.clone())),
            Role::Scheduler => services.spawn(scheduler::start_scheduler(config.clone())),
            Role::Dispatcher => services.spawn(dispatcher::start_dispatcher(config.clone())),
        };
    }

    tokio::signal::ctrl_c().await?;
    info!("Received interrupt signal, terminating servers...");
    services.shutdown().await;

    Ok(())
}
