use shared::netutils;
use tokio::select;
use tracing::{error, info, warn};

use axum::{routing::get, Router};

use shared::service;

#[tracing::instrument(skip_all, fields(service = context.service_name()))]
pub async fn start_api_server(mut context: service::ServiceContext) {
    let config = context.load_config();
    let addr = netutils::parse_addr(config.api.address, config.api.port).unwrap();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));

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
    info!("Received request");
    "Hello, World!"
}
