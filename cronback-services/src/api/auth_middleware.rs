use std::sync::Arc;

use axum::extract::State;
use axum::http::{self, HeaderMap, HeaderValue, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;
use lib::prelude::*;
use tracing::error;

use super::auth::SecretApiKey;
use super::errors::ApiError;
use super::AppState;

fn get_auth_key(
    header_map: &HeaderMap<HeaderValue>,
) -> Result<String, ApiError> {
    let auth_header = header_map
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(ApiError::Unauthorized);
    };

    if auth_header.is_empty() {
        return Err(ApiError::Unauthorized);
    }

    match auth_header.split_once(' ') {
        | Some((name, content)) if name == "Bearer" => Ok(content.to_string()),
        | _ => {
            Err(ApiError::BadRequest(
                "Authentication header is malformed, please use \
                 `Authorization: Bearer sk_...`"
                    .to_owned(),
            ))
        }
    }
}

/// Ensures that the caller is authenticated with an admin key AND acting on
/// behalf of a project. The `ProjectId` is then injected in the request
/// extensions.
pub async fn admin_only_auth_for_project<B>(
    State(state): State<Arc<AppState>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_key = get_auth_key(req.headers())?;
    let admin_keys = &state.config.api.admin_api_keys;
    if admin_keys.contains(&auth_key) {
        let project = extract_project_from_request(&req)?;
        req.extensions_mut().insert(project.clone());
        Ok(next.run(req).await)
    } else {
        Err(ApiError::Forbidden)
    }
}

/// Ensures that the caller is authenticated with an admin key. No project is
/// required. Handlers using this middleware shouldn't rely on a `ProjectId`
/// being set in the request extensions.
pub async fn admin_only_auth<B>(
    State(state): State<Arc<AppState>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_key = get_auth_key(req.headers())?;
    let admin_keys = &state.config.api.admin_api_keys;
    if admin_keys.contains(&auth_key) {
        Ok(next.run(req).await)
    } else {
        Err(ApiError::Forbidden)
    }
}

fn extract_project_from_request<B>(
    req: &Request<B>,
) -> Result<ValidShardedId<ProjectId>, ApiError> {
    // This is an admin user which is acting on behalf of some project.
    const ON_BEHALF_OF_HEADER_NAME: &str = "X-On-Behalf-Of";
    if let Some(project) = req.headers().get(ON_BEHALF_OF_HEADER_NAME) {
        let project = project.to_str().map_err(|_| {
            ApiError::BadRequest(format!(
                "{ON_BEHALF_OF_HEADER_NAME} header is not a valid UTF-8 string"
            ))
        })?;
        let validated_project = ProjectId::from(project.to_owned())
            .validated()
            .map_err(|_| {
                ApiError::BadRequest(format!(
                    "Invalid project id in {ON_BEHALF_OF_HEADER_NAME} header"
                ))
            });
        return validated_project;
    }

    error!("Admin user didn't set {} header", ON_BEHALF_OF_HEADER_NAME);

    Err(ApiError::BadRequest(
        "Super privilege header(s) missing!".to_owned(),
    ))
}

pub async fn auth<B>(
    State(state): State<Arc<AppState>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_key = get_auth_key(req.headers())?;
    let admin_keys = &state.config.api.admin_api_keys;
    if admin_keys.contains(&auth_key) {
        let project = extract_project_from_request(&req)?;
        req.extensions_mut().insert(project.clone());
        return Ok(next.run(req).await);
    }

    let Ok(user_provided_secret) = auth_key.to_string().parse::<SecretApiKey>()
    else {
        return Err(ApiError::Unauthorized);
    };

    let project = state
        .authenicator
        .authenticate(&user_provided_secret)
        .await?;

    req.extensions_mut().insert(project.clone());
    let mut resp = next.run(req).await;
    // Inject project_id in the response extensions as well.
    resp.extensions_mut().insert(project);
    Ok(resp)
}
