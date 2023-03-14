mod get;
mod install;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::post(install::install))
        .route("/:id", axum::routing::get(get::get))
        .with_state(shared_state)
}
