use anyhow::Result;
use shared::config::CoreConfig;
use tracing::info;

use axum::{routing::get, Router};
use std::net::{Ipv6Addr, SocketAddr};

pub async fn start_api_server(config: CoreConfig) -> Result<()> {
    info!("API server listening on :::{:?}", config.api.port);

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));

    let addr = SocketAddr::from((Ipv6Addr::UNSPECIFIED, config.api.port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    info!("Received request");
    "Hello, World!"
}
