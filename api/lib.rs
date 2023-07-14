pub(crate) mod auth;
pub(crate) mod auth_store;
pub mod errors;
pub(crate) mod extractors;
mod handlers;
mod model;

use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use auth_store::AuthStore;
use axum::extract::MatchedPath;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use lib::config::Config;
use lib::database::attempt_log_store::{AttemptLogStore, SqlAttemptLogStore};
use lib::database::invocation_store::{InvocationStore, SqlInvocationStore};
use lib::database::trigger_store::{SqlTriggerStore, TriggerStore};
use lib::database::Database;
use lib::grpc_client_provider::GrpcRequestTracingInterceptor;
use lib::model::{Shard, ValidShardedId};
use lib::types::{ProjectId, RequestId};
use lib::{netutils, service};
use metrics::{histogram, increment_counter};
use proto::scheduler_proto::scheduler_client::SchedulerClient as GenSchedulerClient;
use thiserror::Error;
use tokio::select;
use tonic::codegen::InterceptedService;
use tonic::transport::Endpoint;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::{MakeSpan, TraceLayer};
use tracing::{error, error_span, info, warn};

use crate::auth_store::SqlAuthStore;

#[derive(Debug, Error)]
pub enum AppStateError {
    #[error(transparent)]
    ConnectError(#[from] tonic::transport::Error),
    #[error("Internal data routing error: {0}")]
    RoutingError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

pub struct Db {
    pub trigger_store: Box<dyn TriggerStore + Send + Sync>,
    pub invocation_store: Box<dyn InvocationStore + Send + Sync>,
    pub attempt_store: Box<dyn AttemptLogStore + Send + Sync>,
    pub auth_store: Box<dyn AuthStore + Send + Sync>,
}

pub struct AppState {
    pub _context: service::ServiceContext,
    pub config: Config,
    pub db: Db,
}

pub type SchedulerClient = GenSchedulerClient<
    InterceptedService<
        tonic::transport::Channel,
        GrpcRequestTracingInterceptor,
    >,
>;

impl AppState {
    pub async fn get_scheduler(
        &self,
        request_id: &RequestId,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<SchedulerClient, AppStateError> {
        // For now, we'll assume all triggers are on Cell 0
        // TODO: Use the project's shard to determine which
        // scheduler to use.
        self.scheduler(request_id, project.shard()).await
    }

    pub async fn scheduler(
        &self,
        request_id: &RequestId,
        shard: Shard,
    ) -> Result<SchedulerClient, AppStateError> {
        let address = self
            .config
            .api
            .scheduler_cell_map
            // TODO: Map project shards to scheduler cells
            // For now, we'll assume all triggers are on Cell 0
            .get(&0)
            .ok_or_else(|| {
                AppStateError::RoutingError(format!(
                    "No scheduler found for shard {shard}"
                ))
            })?;
        // TODO: Cache the scheduler channels
        let channel = Endpoint::from_str(address).unwrap().connect().await?;
        Ok(GenSchedulerClient::with_interceptor(
            channel,
            GrpcRequestTracingInterceptor(request_id.clone()),
        ))
    }
}

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_api_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();
    let addr =
        netutils::parse_addr(&config.api.address, config.api.port).unwrap();

    let db = Database::connect(&config.api.database_uri).await?;

    // Only the auth store needs to be prep-ed as it's owned by the API layer.
    // The other stores will be prep-ed by their owner component.
    let auth_store = SqlAuthStore::new(db.clone());
    auth_store.prepare().await?;

    let stores = Db {
        trigger_store: Box::new(SqlTriggerStore::new(db.clone())),
        invocation_store: Box::new(SqlInvocationStore::new(db.clone())),
        attempt_store: Box::new(SqlAttemptLogStore::new(db.clone())),
        auth_store: Box::new(auth_store),
    };

    let shared_state = Arc::new(AppState {
        _context: context.clone(),
        config: config.clone(),
        db: stores,
    });

    let service_name = context.service_name().to_string();
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .nest("/v1", handlers::routes(shared_state.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::any())
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(ApiMakeSpan::new(service_name)),
        )
        .route_layer(middleware::from_fn(track_metrics))
        .fallback(fallback);

    // Handle 404
    let app = app.fallback(handler_404);

    let mut context_clone = context.clone();
    info!("Starting '{}' on {:?}", context.service_name(), addr);
    let server = axum::Server::try_bind(&addr)?;

    let server = server
        .serve(app.into_make_service())
        .with_graceful_shutdown(context.recv_shutdown_signal());

    // Waiting for shutdown signal
    select! {
        _ = context_clone.recv_shutdown_signal() => {
            warn!("Received shutdown signal!");
        },
        res = server => {
        if let Err(e) = res {
            error!(
                "Service '{}' failed and will trigger system shutdown: {e}",
                context.service_name()
            );
            context.broadcast_shutdown();
        }
        }
    };
    Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hey, better visit https://cronback.me"
}

#[derive(Clone, Debug)]
struct ApiMakeSpan {
    service_name: String,
}

impl ApiMakeSpan {
    fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

impl<B> MakeSpan<B> for ApiMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> tracing::Span {
        // We get the request_id from extensions
        let request_id = request
            .extensions()
            .get::<RequestId>()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".into());
        error_span!(
            "http_request",
             // Then we put request_id into the span
             service = self.service_name,
             request_id = %request_id,
             method = %request.method(),
             uri = %request.uri(),
             version = ?request.version(),
        )
    }
}

async fn track_metrics<B>(
    mut req: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    let request_id = RequestId::new();
    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>()
    {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };
    let method = req.method().clone();

    // Inject RequestId into extensions. Can be useful if someone wants to
    // log the request_id
    req.extensions_mut().insert(request_id.clone());

    let mut response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [
        ("method", method.to_string()),
        ("path", path),
        ("status", status),
    ];

    increment_counter!("cronback.api.http_requests_total", &labels);
    histogram!(
        "cronback.api.http_requests_duration_seconds",
        latency,
        &labels
    );
    // Inject request_id into response headers
    response.headers_mut().insert(
        "cronback-request-id",
        request_id.to_string().parse().unwrap(),
    );

    response
}

// handle 404
async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Are you lost, mate?")
}
