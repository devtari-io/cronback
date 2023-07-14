use std::sync::Arc;

use axum::{middleware, Router};

use crate::auth::{admin_only_auth, auth as auth_middleware};
use crate::AppState;

pub(crate) mod admin;
pub(crate) mod runs;
pub(crate) mod triggers;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            "/admin",
            admin::routes(Arc::clone(&shared_state)).route_layer(
                middleware::from_fn_with_state(
                    Arc::clone(&shared_state),
                    admin_only_auth,
                ),
            ),
        )
        .nest(
            "/triggers",
            triggers::routes(Arc::clone(&shared_state)).route_layer(
                middleware::from_fn_with_state(
                    Arc::clone(&shared_state),
                    auth_middleware,
                ),
            ),
        )
        .nest(
            "/runs",
            runs::routes(Arc::clone(&shared_state)).route_layer(
                middleware::from_fn_with_state(
                    Arc::clone(&shared_state),
                    auth_middleware,
                ),
            ),
        )
}
