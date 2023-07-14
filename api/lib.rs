mod api_model;
pub(crate) mod auth;
pub(crate) mod auth_store;
pub mod errors;
pub(crate) mod extractors;
mod handlers;

use std::sync::Arc;
use std::time::Instant;

use auth_store::AuthStore;
use axum::extract::MatchedPath;
use axum::http::{Request, StatusCode};
use axum::middleware::{
    Next,
    {self},
};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use lib::config::Config;
use lib::database::attempt_log_store::{AttemptLogStore, SqlAttemptLogStore};
use lib::database::invocation_store::{InvocationStore, SqlInvocationStore};
use lib::database::trigger_store::{SqlTriggerStore, TriggerStore};
use lib::database::SqliteDatabase;
use lib::types::{CellId, OwnerId, TriggerId};
use lib::{netutils, service};
use metrics::{histogram, increment_counter};
use proto::scheduler_proto::scheduler_client::SchedulerClient as GenSchedulerClient;
use rand::seq::SliceRandom;
use thiserror::Error;
use tokio::select;
use tracing::{error, info, warn};

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
pub type SchedulerClient = GenSchedulerClient<tonic::transport::Channel>;

impl AppState {
    pub async fn pick_scheduler(
        &self,
        _owner_id: String,
    ) -> Result<(CellId, SchedulerClient), AppStateError> {
        let (cell_id, address) = self.pick_random_scheduler();
        Ok((cell_id, SchedulerClient::connect(address).await?))
    }

    pub async fn scheduler(
        &self,
        cell_id: CellId,
    ) -> Result<SchedulerClient, AppStateError> {
        let address = self
            .config
            .api
            .scheduler_cell_map
            .get(&cell_id.0)
            .ok_or_else(|| {
                AppStateError::RoutingError(format!(
                    "No scheduler with cell_id: {cell_id}"
                ))
            })?;
        Ok(SchedulerClient::connect(address.clone()).await?)
    }

    pub async fn scheduler_for_trigger(
        &self,
        _trigger_id: &TriggerId,
    ) -> Result<SchedulerClient, AppStateError> {
        // Decide the scheduler cell
        // TODO: Now, how do we figure which scheduler has this trigger?
        // For now, we'll assume all triggers are on Cell 0
        let cell_id = CellId::from(0);
        self.scheduler(cell_id).await
    }

    pub async fn scheduler_for_owner(
        &self,
        _owner_id: &OwnerId,
    ) -> Result<SchedulerClient, AppStateError> {
        // Decide the scheduler cell
        // For now, we'll assume all triggers are on Cell 0
        let cell_id = CellId::from(0);
        self.scheduler(cell_id).await
    }

    fn pick_random_scheduler(&self) -> (CellId, String) {
        let mut rng = rand::thread_rng();
        // // pick random entry from hashmap self.config.api.scheduler_cell_map
        let keys: Vec<_> = self.config.api.scheduler_cell_map.iter().collect();
        let (cell_id, address) = keys.choose(&mut rng).unwrap();
        info!("Picked scheduler cell {} at {}", cell_id, address);
        (CellId::from(**cell_id), address.to_string())
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

    let db = SqliteDatabase::connect(&config.api.database_uri).await?;

    let stores = Db {
        trigger_store: Box::new(SqlTriggerStore::create(db.clone()).await?),
        invocation_store: Box::new(
            SqlInvocationStore::create(db.clone()).await?,
        ),
        attempt_store: Box::new(SqlAttemptLogStore::create(db.clone()).await?),
        auth_store: Box::new(SqlAuthStore::create(db.clone()).await?),
    };

    let shared_state = Arc::new(AppState {
        _context: context.clone(),
        config: config.clone(),
        db: stores,
    });

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .nest("/v1", handlers::routes(shared_state.clone()))
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
    (StatusCode::NOT_FOUND, "You lost mate?")
}
