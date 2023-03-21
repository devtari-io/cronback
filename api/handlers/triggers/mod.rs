mod get;
mod install;
mod invocations;
mod invoke;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::post(install::install))
        .route("/", axum::routing::get(get::list))
        .route("/:id", axum::routing::get(get::get))
        .route("/:id/invocations", axum::routing::get(invocations::list))
        .route("/:id/invoke", axum::routing::post(invoke::invoke))
        .with_state(shared_state)
}
