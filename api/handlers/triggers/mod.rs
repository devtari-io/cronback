mod cancel;
mod get;
mod install;
mod pause;
mod put;
mod resume;
mod run;
mod runs;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::post(install::install))
        .route("/", axum::routing::get(get::list))
        .route("/:name", axum::routing::get(get::get))
        .route("/:name", axum::routing::put(put::put))
        .route("/:name/runs", axum::routing::get(runs::list))
        .route("/:name/runs/:run_id", axum::routing::get(runs::get))
        .route("/:name/run", axum::routing::post(run::run))
        .route("/:name/pause", axum::routing::post(pause::pause))
        .route("/:name/cancel", axum::routing::post(cancel::cancel))
        .route("/:name/resume", axum::routing::post(resume::resume))
        .with_state(shared_state)
}
