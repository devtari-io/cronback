mod cli;
mod metric_defs;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use shared::{
    config::{ConfigLoader, Role},
    netutils::parse_addr,
    service::ServiceContext,
    shutdown::Shutdown,
};
use tokio::{select, task::JoinSet, time};
use tracing::{error, info, trace, warn};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Shutdown broadcast channel first
    let opts = cli::CliOpts::parse();
    let mut shutdown = Shutdown::default();
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
    let prometheus_sockaddr = parse_addr(
        config.main.prometheus_address,
        config.main.prometheus_port,
    )?;
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
    for ref role in config.main.roles {
        services.spawn(spawn_service(
            role.clone(),
            config_loader.clone(),
            shutdown.clone(),
        ));
    }

    // Waiting for <C-c> to terminate
    select! {
        _ = shutdown.recv() => {
            warn!("Received shutdown signal from downstream services!");
        },
        _ = tokio::signal::ctrl_c() => {
            warn!("Received Ctrl+c signal (SIGINT)!");
            shutdown.broadcast_shutdown();
        }
    };

    // Give services 10 seconds to cleanly shutdown after the shutdown signal.
    info!("Waiting (10s) for services to shutdown cleanly...");
    if (time::timeout(Duration::from_secs(10), async {
        while services.join_next().await.is_some() {
            info!("Need to wait for {} services to terminate", services.len());
        }
    })
    .await)
        .is_err()
    {
        error!(
            "Timed out awaiting {} services to shutdown!",
            services.len()
        );
        services.shutdown().await;
        bail!("Some services were not terminated cleanly!");
    }
    info!("Bye!");

    Ok(())
}

async fn spawn_service(
    role: Role,
    config_loader: Arc<ConfigLoader>,
    mut shutdown: Shutdown,
) {
    let service_name = format!("{role:?}");
    info!(service = service_name, "Starting service '{service_name}'");

    let join_handle = match role {
        | Role::Api => {
            tokio::spawn(api::start_api_server(ServiceContext::new(
                service_name.clone(),
                config_loader,
                shutdown.clone(),
            )))
        }
        | Role::Scheduler => tokio::spawn(scheduler::start_scheduler_server(
            ServiceContext::new(
                service_name.clone(),
                config_loader,
                shutdown.clone(),
            ),
        )),
        | Role::Dispatcher => tokio::spawn(
            dispatcher::start_dispatcher_server(ServiceContext::new(
                service_name.clone(),
                config_loader,
                shutdown.clone(),
            )),
        ),
    };
    match join_handle.await.unwrap() {
        | Ok(_) => info!("Service '{service_name}' terminated!"),
        | Err(e) => {
            error!("Failed to start '{service_name}': {e}");
            shutdown.broadcast_shutdown();
        }
    }
}
