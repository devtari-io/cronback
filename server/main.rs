mod cli;
mod metric_defs;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use shared::{
    config::{ConfigLoader, Role},
    netutils::parse_addr,
};
use tokio::task::JoinSet;
use tracing::{info, trace, warn};
use tracing_subscriber::FmtSubscriber;
use valuable::Valuable;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = cli::CliOpts::parse();

    let sub = FmtSubscriber::builder()
        .pretty()
        .with_thread_names(true)
        // TODO: Configure logging from command line
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(sub)?;

    info!("** {} **", "CronBack.me".magenta());
    trace!(config = opts.config, "Loading configuration");
    let config_loader = Arc::new(ConfigLoader::from_path(&opts.config));

    // Load initial configuration
    let config = config_loader.load()?;

    // Configure Metric Exporter
    let prometheus_sockaddr =
        parse_addr(config.main.prometheus_address, config.main.prometheus_port)?;
    let builder = PrometheusBuilder::new();
    info!("Prometheus HTTP listener on {:?}", prometheus_sockaddr);
    builder
        .idle_timeout(
            MetricKindMask::HISTOGRAM,
            // Remove a metric from registry if it was not updated for 2 minutes.
            Some(Duration::from_secs(120)),
        )
        .with_http_listener(prometheus_sockaddr)
        .install()
        .expect("failed to install Prometheus recorder");
    // Install metric definitions
    metric_defs::install_metrics();

    // Init services
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
