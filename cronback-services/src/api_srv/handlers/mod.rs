use std::sync::Arc;

use axum::{middleware, Router};

use crate::api_srv::auth_middleware::ensure_authenticated;
use crate::api_srv::AppState;

pub(crate) mod admin;
pub(crate) mod triggers;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/admin", admin::routes(Arc::clone(&shared_state)))
        .nest(
            "/triggers",
            triggers::routes(Arc::clone(&shared_state))
                .route_layer(middleware::from_fn(ensure_authenticated)),
        )
}
