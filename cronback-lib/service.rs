use std::collections::HashSet;
use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use hyper::{Body, Request, Response};
use proto::FILE_DESCRIPTOR_SET;
use sea_orm::ConnectOptions;
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::body::BoxBody;
use tonic::transport::server::{Connected, TcpIncoming};
use tonic::transport::{NamedService, Server as TonicServer};
use tonic_reflection::server::Builder;
use tower::Service;
use tower_http::trace::{MakeSpan, TraceLayer};
use tracing::{error, error_span, info, Id, Span};

use crate::config::Config;
use crate::consts::{PARENT_SPAN_HEADER, PROJECT_ID_HEADER, REQUEST_ID_HEADER};
use crate::database::{Database, DatabaseError, DbMigration};
use crate::rpc_middleware::CronbackRpcMiddleware;
use crate::shutdown::Shutdown;
use crate::MainConfig;

#[async_trait]
pub trait CronbackService: Send + Sync + Sized + Clone + 'static {
    type ServiceConfig: for<'a> Deserialize<'a>
        + Clone
        // TODO: Consider better option instead of this bound.
        + Into<ConnectOptions>
        + Send
        + Sync;
    type Migrator: DbMigration;

    /// The role of the service. This must be unique across all services running
    /// on the same binary. A role can be enabled or disabled via
    /// `main.roles` list in the config file.
    const ROLE: &'static str;
    /// Default config section to the role name (e.g. `scheduler` which
    /// translates to `[scheduler]`) default configuration **must** be in TOML
    /// format.
    const CONFIG_SECTION: &'static str = Self::ROLE;

    /// An additional configuration layer that will be added to the default
    /// configuration _before_ loading any configuration file externally.
    const DEFAULT_CONFIG_TOML: &'static str = "";

    /// Create a new service context.
    fn make_context(
        config: Config,
        shutdown: Shutdown,
    ) -> ServiceContext<Self> {
        ServiceContext::new(config, shutdown)
    }

    // The list of keys in this service configs that should be parsed
    // as vectors.
    fn config_vec_keys() -> HashSet<String> {
        HashSet::default()
    }

    /// Optional hook to install telemetry for the service.
    fn install_telemetry() {}

    /// Create and migrate database before service is started. Return None if no
    /// database is needed.
    async fn prepare_database<O>(opts: O) -> Result<Database, DatabaseError>
    where
        O: Into<ConnectOptions> + Send,
    {
        Database::connect::<O, Self::Migrator>(opts).await
    }

    // Creates and migrate an in-memory database for testing.
    async fn in_memory_database() -> Result<Database, DatabaseError> {
        Database::connect::<&str, Self::Migrator>("sqlite::memory:").await
    }

    async fn serve(
        context: ServiceContext<Self>,
        db: Database,
    ) -> anyhow::Result<()>;
}

// needs to be parametric by config type.
#[derive(Clone)]
pub struct ServiceContext<S> {
    config: Config,
    shutdown: Shutdown,
    _service: std::marker::PhantomData<S>,
}

impl<S> ServiceContext<S>
where
    S: CronbackService,
{
    fn new(config: Config, shutdown: Shutdown) -> Self {
        Self {
            config,
            shutdown,
            _service: Default::default(),
        }
    }

    pub fn service_name(&self) -> &'static str {
        S::ROLE
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn get_main_config(&self) -> MainConfig {
        self.config.get_main()
    }

    /// Awaits the shutdown signal
    pub async fn recv_shutdown_signal(&mut self) {
        self.shutdown.recv().await
    }

    /// Causes all listeners to start the shutdown sequence.
    pub fn broadcast_shutdown(&mut self) {
        self.shutdown.broadcast_shutdown()
    }

    pub fn service_config(&self) -> S::ServiceConfig {
        self.config.get(S::CONFIG_SECTION)
    }
}

// Ensure that ServiceContext is Send + Sync
const _: () = {
    struct DummyService;
    const fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<ServiceContext<DummyService>>();
};

#[derive(Clone, Debug)]
struct GrpcMakeSpan {
    service_name: String,
}
impl GrpcMakeSpan {
    fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

impl<B> MakeSpan<B> for GrpcMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let request_id = request
            .headers()
            .get(REQUEST_ID_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());

        let parent_span = request
            .headers()
            .get(PARENT_SPAN_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());

        let project_id = request
            .headers()
            .get(PROJECT_ID_HEADER)
            .map(|v| v.to_str().unwrap().to_owned());

        let span = error_span!(
            "grpc_request",
             service = %self.service_name,
             request_id = %request_id.unwrap_or_default(),
             project_id = %project_id.unwrap_or_default(),
             method = %request.method(),
             uri = %request.uri(),
             version = ?request.version(),
        );

        if let Some(parent_span) = parent_span {
            let id = Id::from_u64(parent_span.parse().unwrap());
            span.follows_from(id);
        }
        span
    }
}

pub async fn grpc_serve_tcp<S, CS>(
    context: &mut ServiceContext<CS>,
    addr: SocketAddr,
    svc: S,
    timeout: u64,
) where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    CS: CronbackService,
{
    info!("Starting '{}' on {:?}", context.service_name(), addr);
    match TcpIncoming::new(addr, true, None) {
        | Ok(incoming) => {
            grpc_serve_incoming(context, svc, incoming, timeout).await
        }
        | Err(e) => {
            error!(
                "RPC service '{}' couldn't bind on address '{addr}', system \
                 will shutdown: {e}",
                context.service_name()
            );
            context.broadcast_shutdown();
        }
    };
}

pub async fn grpc_serve_unix<S, K, CS>(
    context: &mut ServiceContext<CS>,
    socket: K,
    svc: S,
    timeout: u64,
) where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    K: AsRef<Path>,
    CS: CronbackService,
{
    info!(
        "Starting '{}' on {:?}",
        context.service_name(),
        socket.as_ref()
    );
    let uds = UnixListener::bind(socket).unwrap();
    let stream = UnixListenerStream::new(uds);
    grpc_serve_incoming(context, svc, stream, timeout).await
}

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
async fn grpc_serve_incoming<S, K, IO, IE, CS>(
    context: &mut ServiceContext<CS>,
    svc: S,
    incoming: K,
    timeout: u64,
) where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    K: Stream<Item = Result<IO, IE>>,
    IO: AsyncRead + AsyncWrite + Connected + Unpin + Send + 'static,
    IO::ConnectInfo: Clone + Send + Sync + 'static,
    IE: Into<Box<dyn Error + Send + Sync>>,
    CS: CronbackService,
{
    let svc_name = context.service_name().to_owned();
    // The stack of middleware that our service will be wrapped in
    let cronback_middleware = tower::ServiceBuilder::new()
        // Apply our own middleware
        .layer(
            TraceLayer::new_for_grpc()
                .make_span_with(GrpcMakeSpan::new(svc_name)),
        )
        .layer(CronbackRpcMiddleware::new(context.service_name()))
        .into_inner();

    let reflection_service = Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    // grpc Server
    if let Err(e) = TonicServer::builder()
        .timeout(Duration::from_secs(timeout))
        .layer(cronback_middleware)
        .add_service(reflection_service)
        .add_service(svc)
        .serve_with_incoming_shutdown(incoming, context.recv_shutdown_signal())
        .await
    {
        error!(
            "RPC service '{}' failed to start and will trigger system \
             shutdown: {e}",
            context.service_name()
        );
        context.broadcast_shutdown()
    } else {
        info!("Service '{}' terminated", context.service_name());
    }
}
