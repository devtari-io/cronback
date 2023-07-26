pub mod cli;
mod metric_defs;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Parser;
use cli::LogFormat;
use colored::Colorize;
use lib::netutils::parse_addr;
use lib::prelude::*;
use lib::{ConfigLoader, MainConfig, Shutdown};
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
        let env_filter = tracing_subscriber::EnvFilter::builder()
            .with_env_var("CRONBACK_LOG")
            .try_from_env()
            .unwrap_or_else(|_| {
                "info,sqlx=warn,cronbackd=debug,cronback_services=debug,\
                 tower_http=info,cronback_lib=debug,\
                 request_response_tracing=off,\
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

async fn prepare_database<S: CronbackService>(
    ctx: &ServiceContext<S>,
) -> Result<Database, anyhow::Error> {
    let service_name = ctx.service_name();
    info!(service = service_name, "Preparing database");
    // We load the service configuration to ensure that we have a good
    // config.
    let config = ctx.load_service_config().with_context(|| {
        format!("Failed to load configuration for service '{service_name}")
    })?;

    // Service config must implement Into<ConnectOptions>
    S::prepare_database(config).await.with_context(|| {
        format!("Failed to prepare database for service '{service_name}")
    })
}

async fn spawn_service<S: CronbackService>(
    mut ctx: ServiceContext<S>,
    db: Database,
) {
    let service_name = ctx.service_name();
    info!(service = service_name, "Starting service");
    let res = S::serve(ctx.clone(), db).await;
    match res {
        | Ok(_) => info!(service = service_name, "Service terminated!"),
        | Err(e) => {
            error!(service = service_name, "Failed to start service: {e}");
            ctx.broadcast_shutdown();
        }
    }
}

fn print_banner() {
    debug!("** {} **", "cronback.me".magenta());
}

fn setup_prometheus(config: &MainConfig) -> Result<()> {
    // Configure Metric Exporter
    let prometheus_sockaddr =
        parse_addr(&config.prometheus_address, config.prometheus_port)?;
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

    Ok(())
}

mod private {
    // Prevents downstream users from implementing the `Cronback` trait.
    pub trait Sealed {}
}

#[async_trait]
pub trait Cronback: private::Sealed {
    async fn run_cronback() -> Result<()>;
}

macro_rules! impl_cronback_with {
     ( $($ty:ident),* $(,)? ) => {

        impl<$($ty,)*> private::Sealed for ($($ty,)*)
            where
             $($ty: CronbackService),* {}

         #[allow(non_snake_case)]
         #[allow(unused_variables)]
         #[allow(unused_mut)]
         #[async_trait]
         impl<$($ty,)*> Cronback for ($($ty,)*) where
             $($ty: CronbackService),*
         {
            async fn run_cronback() -> Result<()> {
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

                print_banner();
                trace!(config = opts.config, "Loading configuration");
                let config_loader = Arc::new(ConfigLoader::from_path(&opts.config));
                let config_main = config_loader.load_main()?;

                // Configure Metric Exporter
                setup_prometheus(&config_main)?;

                // Install metric definitions
                metric_defs::install_metrics();

                // Init services
                let mut available_roles: HashSet<String> = HashSet::new();
                let mut services: JoinSet<()> = JoinSet::new();
                $(
                    if !available_roles.insert($ty::ROLE.to_string()) {
                        // We have two services registering the same role name!
                        bail!("Cannot register service ({}). Role '{}' has been already registered by another service!",
                             std::any::type_name::<$ty>(),
                              $ty::ROLE);
                    }

                    let $ty = $ty::make_context(config_loader.clone(), shutdown.clone());
                )*

                if !config_main.roles.is_subset(&available_roles) {
                    bail!("Unrecognized service roles were found in the config: {:?}",
                          config_main.roles.difference(&available_roles));
                }

                info!("Initializing services");
                // Initialise services and run database migrations before serving any traffic on
                // any service.
                let mut databases: HashMap<String, Database> = HashMap::new();
                $(
                    if config_main.roles.contains($ty::ROLE)
                    {
                        debug!(service = $ty::ROLE, "Installing telemetry");
                        $ty::install_telemetry();
                        databases.insert($ty::ROLE.to_owned(), prepare_database(&$ty).await?);
                    }
                )*

                info!("Services has completed database migrations");
                // spawn the services in the order there were registered
                $(
                    if config_main.roles.contains($ty::ROLE)
                    {
                        services.spawn(spawn_service($ty, databases.remove($ty::ROLE).unwrap()));
                    }
                )*

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
         }
     };
}

// Add more if more services are needed.
impl_cronback_with!();
impl_cronback_with!(A);
impl_cronback_with!(A, B);
impl_cronback_with!(A, B, C);
impl_cronback_with!(A, B, C, D);
impl_cronback_with!(A, B, C, D, E);
impl_cronback_with!(A, B, C, D, E, F);
impl_cronback_with!(A, B, C, D, E, F, G);
impl_cronback_with!(A, B, C, D, E, F, G, H);
impl_cronback_with!(A, B, C, D, E, F, G, H, I);
impl_cronback_with!(A, B, C, D, E, F, G, H, I, J);
impl_cronback_with!(A, B, C, D, E, F, G, H, I, J, K);
impl_cronback_with!(A, B, C, D, E, F, G, H, I, J, K, L);
