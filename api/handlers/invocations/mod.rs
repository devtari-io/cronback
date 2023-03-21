mod attempts;
mod get;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(get::list))
        .route("/:id", axum::routing::get(get::get))
        .route("/:id/attempts", axum::routing::get(attempts::list))
        .with_state(shared_state)
}
