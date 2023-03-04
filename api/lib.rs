pub mod errors;
mod handlers;
mod model;

use std::{sync::Arc, time::Instant};

use metrics::{histogram, increment_counter};
use shared::netutils;
use tokio::select;
use tracing::{error, info, warn};

use axum::{
    extract::MatchedPath,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
    routing::post,
    Router,
};

use handlers::create_trigger::create_trigger;
use shared::service;

pub(crate) struct AppState {}

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_api_server(mut context: service::ServiceContext) {
    let config = context.load_config();
    let addr =
        netutils::parse_addr(config.api.address, config.api.port).unwrap();

    let shared_state = Arc::new(AppState {});

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/v1/triggers", post(create_trigger))
        .route_layer(middleware::from_fn(track_metrics))
        .with_state(shared_state);

    // Handle 404
    let app = app.fallback(handler_404);

    let mut context_clone = context.clone();
    info!("Starting '{}' on {:?}", context.service_name(), addr);
    let server = axum::Server::try_bind(&addr);
    if let Err(e) = server {
        error!(
            "Service '{}' failed to start and will trigger system shutdown: {}",
            context.service_name(),
            e
        );
        context.broadcast_shutdown();
        return;
    }

    let server = server
        .unwrap()
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
