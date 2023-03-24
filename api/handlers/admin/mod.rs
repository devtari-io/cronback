mod api_key;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api_key", axum::routing::post(api_key::create))
        .with_state(shared_state)
}
