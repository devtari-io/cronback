use std::sync::Arc;

use axum::{middleware, Router};

use crate::auth::auth as auth_middleware;
use crate::AppState;

pub(crate) mod admin;
pub(crate) mod triggers;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/admin", admin::routes(Arc::clone(&shared_state)))
        .nest(
            "/triggers",
            triggers::routes(Arc::clone(&shared_state)).route_layer(
                middleware::from_fn_with_state(
                    Arc::clone(&shared_state),
                    auth_middleware,
                ),
            ),
        )
}
