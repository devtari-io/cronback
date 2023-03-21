use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub(crate) mod invocations;
pub(crate) mod triggers;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/triggers", triggers::routes(Arc::clone(&shared_state)))
        .nest("/invocations", invocations::routes(shared_state))
}
