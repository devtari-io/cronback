mod cancel;
mod get;
mod install;
mod invocations;
mod invoke;
mod pause;
mod put;
mod resume;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::post(install::install))
        .route("/", axum::routing::get(get::list))
        .route("/:id", axum::routing::get(get::get))
        .route("/:id", axum::routing::put(put::put))
        .route("/:id/invocations", axum::routing::get(invocations::list))
        .route("/:id/invoke", axum::routing::post(invoke::invoke))
        .route("/:id/pause", axum::routing::post(pause::pause))
        .route("/:id/cancel", axum::routing::post(cancel::cancel))
        .route("/:id/resume", axum::routing::post(resume::resume))
        .with_state(shared_state)
}
