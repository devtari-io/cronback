use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) mod triggers;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new().nest("/triggers", triggers::routes(shared_state.clone()))
}
