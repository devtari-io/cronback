use std::sync::Arc;

use anyhow::Result;
use shared::config::ConfigLoader;
use shared::netutils;
use tracing::info;

use axum::{routing::get, Router};

pub async fn start_api_server(config_loader: Arc<ConfigLoader>) -> Result<()> {
    // _almost_ guaranteed to succeed!
    let config = config_loader.load()?;
    let addr = netutils::parse_addr(&config.api.address, config.api.port)?;
    info!("API server listening on {:?}", addr);

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));

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
