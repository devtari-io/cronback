mod api_keys;
mod projects;

use std::sync::Arc;

use axum::{middleware, Router};

use crate::auth_middleware::{admin_only_auth, admin_only_auth_for_project};
use crate::AppState;

pub(crate) fn routes(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            "/api_keys",
            Router::new()
                .route("/", axum::routing::post(api_keys::create))
                .route("/", axum::routing::get(api_keys::list))
                .route("/:id", axum::routing::delete(api_keys::revoke))
                .with_state(Arc::clone(&shared_state))
                .route_layer(middleware::from_fn_with_state(
                    Arc::clone(&shared_state),
                    admin_only_auth_for_project,
                )),
        )
        .nest(
            "/projects",
            Router::new()
                .route("/", axum::routing::post(projects::create))
                .route("/:id/disable", axum::routing::post(projects::disable))
                .route("/:id/enable", axum::routing::post(projects::enable))
                .with_state(Arc::clone(&shared_state))
                .route_layer(middleware::from_fn_with_state(
                    Arc::clone(&shared_state),
                    admin_only_auth,
                )),
        )
}
