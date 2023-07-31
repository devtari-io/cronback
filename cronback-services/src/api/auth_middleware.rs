use std::sync::Arc;

use axum::extract::State;
use axum::http::{self, HeaderMap, HeaderValue, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;
use lib::prelude::*;

use super::auth::{AuthError, SecretApiKey};
use super::errors::ApiError;
use super::AppState;

const ON_BEHALF_OF_HEADER_NAME: &str = "X-On-Behalf-Of";

enum AuthenticationStatus {
    Unauthenticated,
    Authenticated(ValidShardedId<ProjectId>),
    Admin(Option<ValidShardedId<ProjectId>>),
}

impl AuthenticationStatus {
    fn project_id(&self) -> Option<ValidShardedId<ProjectId>> {
        match self {
            | AuthenticationStatus::Authenticated(p) => Some(p.clone()),
            | AuthenticationStatus::Admin(Some(p)) => Some(p.clone()),
            | _ => None,
        }
    }
}

/// Parses the AUTHORIZATION header to extract the user provided secret key.
fn get_auth_key(
    header_map: &HeaderMap<HeaderValue>,
) -> Result<Option<String>, ApiError> {
    let auth_header = header_map
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Ok(None);
    };

    if auth_header.is_empty() {
        return Ok(None);
    }

    match auth_header.split_once(' ') {
        | Some((name, content)) if name == "Bearer" => {
            Ok(Some(content.to_string()))
        }
        | _ => {
            Err(ApiError::BadRequest(
                "Authentication header is malformed, please use \
                 `Authorization: Bearer sk_...`"
                    .to_owned(),
            ))
        }
    }
}

async fn get_auth_status<B>(
    state: &AppState,
    req: &Request<B>,
) -> Result<AuthenticationStatus, ApiError> {
    let auth_key = get_auth_key(req.headers())?;
    let Some(auth_key) = auth_key else {
        return Ok(AuthenticationStatus::Unauthenticated);
    };
    let config = state.context.service_config();
    let admin_keys = &config.admin_api_keys;
    if admin_keys.contains(&auth_key) {
        let project: Option<ValidShardedId<ProjectId>> = req
            .headers()
            .get(ON_BEHALF_OF_HEADER_NAME)
            .map(HeaderValue::to_str)
            .transpose()
            .map_err(|_| {
                ApiError::BadRequest(format!(
                    "{ON_BEHALF_OF_HEADER_NAME} header is not a valid UTF-8 \
                     string"
                ))
            })?
            .map(|p| ProjectId::from(p.to_owned()).validated())
            .transpose()
            .map_err(|_| {
                ApiError::BadRequest(format!(
                    "Invalid project id in {ON_BEHALF_OF_HEADER_NAME} header"
                ))
            })?;

        return Ok(AuthenticationStatus::Admin(project));
    }

    let Ok(user_provided_secret) = auth_key.to_string().parse::<SecretApiKey>()
    else {
        return Ok(AuthenticationStatus::Unauthenticated);
    };

    let project = state.authenicator.authenticate(&user_provided_secret).await;
    match project {
        | Ok(project_id) => Ok(AuthenticationStatus::Authenticated(project_id)),
        | Err(AuthError::AuthFailed(_)) => {
            Ok(AuthenticationStatus::Unauthenticated)
        }
        | Err(e) => {
            tracing::error!("{}", e);
            Err(ApiError::ServiceUnavailable)
        }
    }
}

/// Ensures that the caller is authenticated with a project id.
pub async fn ensure_authenticated<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth = req.extensions().get::<AuthenticationStatus>().expect(
        "All endpoints should have passed by the authentication middleware",
    );
    match auth {
        | AuthenticationStatus::Admin(None) => {
            Err(ApiError::BadRequest(
                "Super privilege header(s) missing!".to_owned(),
            ))
        }
        | AuthenticationStatus::Authenticated(_)
        | AuthenticationStatus::Admin(Some(_)) => Ok(next.run(req).await),
        | AuthenticationStatus::Unauthenticated => Err(ApiError::Unauthorized),
    }
}

/// Ensures that the caller is authenticated with an admin key AND acting on
/// behalf of a project.
pub async fn ensure_admin_for_project<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth = req.extensions().get::<AuthenticationStatus>().expect(
        "All endpoints should have passed by the authentication middleware",
    );

    match auth {
        | AuthenticationStatus::Admin(Some(_)) => Ok(next.run(req).await),
        | AuthenticationStatus::Admin(None) => {
            Err(ApiError::BadRequest(
                "Super privilege header(s) missing!".to_owned(),
            ))
        }
        | AuthenticationStatus::Authenticated(_) => Err(ApiError::Forbidden),
        | AuthenticationStatus::Unauthenticated => Err(ApiError::Unauthorized),
    }
}

/// Ensures that the caller is authenticated with an admin key. No project is
/// required. Handlers using this middleware shouldn't rely on a `ProjectId`
/// being set in the request extensions.
pub async fn ensure_admin<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth = req.extensions().get::<AuthenticationStatus>().expect(
        "All endpoints should have passed by the authentication middleware",
    );

    match auth {
        | AuthenticationStatus::Admin(_) => Ok(next.run(req).await),
        | AuthenticationStatus::Authenticated(_) => Err(ApiError::Forbidden),
        | AuthenticationStatus::Unauthenticated => Err(ApiError::Unauthorized),
    }
}

/// Parses the request headers to extract authentication information. The
/// AuthenticationStatus is then injected in the request/response extensions
/// along with the authenticated ProjectId if found. This middleware only fails
/// if the user passes malformed authentication headers. It's the responsibility
/// of the other "ensure_*" middlewares in this module to enforce the expected
/// AuthenticationStatus for a certain route.
pub async fn authenticate<B>(
    State(state): State<Arc<AppState>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_status = get_auth_status(state.as_ref(), &req).await?;

    let project_id = auth_status.project_id();
    req.extensions_mut().insert(auth_status);

    if let Some(project_id) = &project_id {
        req.extensions_mut().insert(project_id.clone());
    }

    let mut resp = next.run(req).await;

    if let Some(project_id) = &project_id {
        // Inject project_id in the response extensions as well.
        resp.extensions_mut().insert(project_id.clone());
    }

    Ok(resp)
}
