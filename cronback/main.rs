mod cli;
mod metric_defs;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use clap::Parser;
use cli::LogFormat;
use colored::Colorize;
use lib::config::{ConfigLoader, Role};
use lib::netutils::parse_addr;
use lib::service::ServiceContext;
use lib::shutdown::Shutdown;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use tokio::task::JoinSet;
use tokio::{select, time};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Layer};

fn setup_logging_subscriber(
    f: &LogFormat,
    api_tracing_dir: &str,
) -> Vec<tracing_appender::non_blocking::WorkerGuard> {
    let mut guards = vec![];

    // The default stdout logging
    let stdout_layer = {
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| {
                "cronbackd=debug,scheduler=debug,api=debug,dispatcher=debug,\
                 tower_http=debug,lib=debug,request_response_tracing=off,\
                 request_response_tracing_metadata=info,events=off"
                    .into()
            });

        let stdout_layer =
            tracing_subscriber::fmt::layer().with_thread_names(true);
        let stdout_layer: Box<dyn Layer<_> + Send + Sync> = match f {
            | cli::LogFormat::Pretty => stdout_layer.pretty().boxed(),
            | cli::LogFormat::Compact => stdout_layer.compact().boxed(),
            | cli::LogFormat::Json => stdout_layer.json().boxed(),
        };
        stdout_layer.with_filter(env_filter)
    };

    // A special subscriber to separate the request/response tracing to a
    // separate log
    let (request_tracing_layer, file_guard) = {
        let file_appender = tracing_appender::rolling::daily(
            api_tracing_dir,
            "cronback_requests.log",
        );
        let (non_blocking, guard) =
            tracing_appender::non_blocking(file_appender);
        (
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_filter(EnvFilter::new(
                    "off,request_response_tracing=info,\
                     request_response_tracing_metadata=info",
                )),
            guard,
        )
    };
    guards.push(file_guard);
    // A special subscriber for events.
    // TODO: This setup is temporary until we decide how this will be configured
    // and whether we should use rolling file or not.
    let (events_layer, file_guard) = {
        let file_appender = tracing_appender::rolling::daily(
            api_tracing_dir,
            "cronback_events.log",
        );
        let (non_blocking, guard) =
            tracing_appender::non_blocking(file_appender);
        (
            tracing_subscriber::fmt::layer()
                // For events, we only log the message. The message is JSON
                // serialized already.
                .event_format(lib::events::Formatter)
                .with_writer(non_blocking)
                .with_filter(EnvFilter::new("off,events=info")),
            guard,
        )
    };
    guards.push(file_guard);
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(request_tracing_layer)
        .with(events_layer)
        .init();

    guards
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // Load .env file if it exists
    match dotenvy::dotenv() {
        | Ok(_) => {}
        // .env files are optional
        | Err(e) if e.not_found() => {}
        | Err(e) => bail!("Failed to load .env file: {e}"),
    };

    // Shutdown broadcast channel first
    let opts = cli::CliOpts::parse();
    let mut shutdown = Shutdown::default();

    let _tracing_file_guard =
        setup_logging_subscriber(&opts.log_format, &opts.api_tracing_dir);

    debug!("** {} **", "cronback.me".magenta());
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
            // Remove a metric from registry if it was not updated for 2
            // minutes.
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
        | Role::Scheduler => {
            tokio::spawn(scheduler::start_scheduler_server(
                ServiceContext::new(
                    service_name.clone(),
                    config_loader,
                    shutdown.clone(),
                ),
            ))
        }
        | Role::Dispatcher => {
            tokio::spawn(dispatcher::start_dispatcher_server(
                ServiceContext::new(
                    service_name.clone(),
                    config_loader,
                    shutdown.clone(),
                ),
            ))
        }
        | Role::ProjectStore => {
            tokio::spawn(project_srv::start_project_store_server(
                ServiceContext::new(
                    service_name.clone(),
                    config_loader,
                    shutdown.clone(),
                ),
            ))
        }
    };
    match join_handle.await.unwrap() {
        | Ok(_) => info!("Service '{service_name}' terminated!"),
        | Err(e) => {
            error!("Failed to start '{service_name}': {e}");
            shutdown.broadcast_shutdown();
        }
    }
}
