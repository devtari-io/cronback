mod api_model;
pub(crate) mod auth;
pub(crate) mod auth_middleware;
pub(crate) mod auth_store;
mod db_model;
pub mod errors;
pub(crate) mod extractors;
mod handlers;
mod logging;
mod migration;
mod paginated;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use auth::Authenticator;
use auth_store::SqlAuthStore;
use axum::extract::MatchedPath;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use lib::clients::{
    ScopedDispatcherSvcClient,
    ScopedMetadataSvcClient,
    ScopedSchedulerSvcClient,
};
use lib::prelude::*;
use lib::{netutils, service, Config, GrpcClientFactory, GrpcClientProvider};
use logging::{trace_request_response, ApiMakeSpan};
use metrics::{histogram, increment_counter};
use sea_orm::TransactionTrait;
use sea_orm_migration::MigratorTrait;
use thiserror::Error;
use tokio::select;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum AppStateError {
    #[error(transparent)]
    ConnectError(#[from] tonic::transport::Error),
    #[error("Internal data routing error: {0}")]
    RoutingError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

pub struct AppState {
    pub _context: service::ServiceContext,
    pub config: Config,
    pub authenicator: Authenticator,
    pub scheduler_clients:
        Box<dyn GrpcClientFactory<ClientType = ScopedSchedulerSvcClient>>,
    pub dispatcher_clients:
        Box<dyn GrpcClientFactory<ClientType = ScopedDispatcherSvcClient>>,
    pub metadata_svc_clients:
        Box<dyn GrpcClientFactory<ClientType = ScopedMetadataSvcClient>>,
}

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

// TODO: Move database migration into a new service trait.
pub async fn migrate_up(db: &Database) -> Result<(), DatabaseError> {
    let conn = db.orm.begin().await?;
    migration::Migrator::up(&conn, None).await?;
    conn.commit().await?;
    Ok(())
}

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_api_server(
    mut context: service::ServiceContext,
) -> anyhow::Result<()> {
    let config = context.load_config();
    let addr =
        netutils::parse_addr(&config.api.address, config.api.port).unwrap();

    let db = Database::connect(&config.api.database_uri).await?;
    migrate_up(&db).await?;

    let shared_state = Arc::new(AppState {
        _context: context.clone(),
        config: config.clone(),
        authenicator: Authenticator::new(Box::new(SqlAuthStore::new(
            db.clone(),
        ))),
        scheduler_clients: Box::new(GrpcClientProvider::new(context.clone())),
        dispatcher_clients: Box::new(GrpcClientProvider::new(context.clone())),
        metadata_svc_clients: Box::new(GrpcClientProvider::new(
            context.clone(),
        )),
    });

    let service_name = context.service_name().to_string();
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .nest("/v1", handlers::routes(shared_state.clone()))
        .layer(middleware::from_fn_with_state(
            Arc::new(config.clone()),
            trace_request_response,
        ))
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
        .layer(middleware::from_fn_with_state(
            Arc::clone(&shared_state),
            auth_middleware::authenticate,
        ))
        .route_layer(middleware::from_fn(inject_request_id))
        .route_layer(middleware::from_fn(track_metrics))
        .fallback(fallback);

    // Handle 404
    let app = app.fallback(handler_404);

    let mut context_clone = context.clone();
    info!("Starting '{}' on {:?}", context.service_name(), addr);
    let server = axum::Server::try_bind(&addr)?;

    let server = server
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
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

async fn inject_request_id<B>(
    mut req: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    let request_id = RequestId::new();
    // Inject RequestId into extensions. Can be useful if someone wants to
    // log the request_id
    req.extensions_mut().insert(request_id.clone());
    // Run the next layer
    let mut response = next.run(req).await;
    // Inject request_id into response headers
    response
        .headers_mut()
        .insert(REQUEST_ID_HEADER, request_id.to_string().parse().unwrap());

    // Inject project_id into response headers
    if let Some(project_id) = response
        .extensions()
        .get::<ValidShardedId<ProjectId>>()
        .cloned()
    {
        response
            .headers_mut()
            .insert(PROJECT_ID_HEADER, project_id.to_string().parse().unwrap());
    }
    response
}

async fn track_metrics<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>()
    {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };
    let method = req.method().clone();

    let response = next.run(req).await;

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

    response
}

// handle 404
async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Are you lost, mate?")
}
