mod api_keys;
mod projects;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api_keys", axum::routing::post(api_keys::create))
        .route("/api_keys", axum::routing::get(api_keys::list))
        .route("/api_keys/:id", axum::routing::delete(api_keys::revoke))
        .route("/projects", axum::routing::post(projects::create))
        .with_state(shared_state)
}
