use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::http::{self, HeaderMap, HeaderValue, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;
use lib::prelude::*;

use super::auth::{AuthError, Authenticator, SecretApiKey};
use super::errors::ApiError;
use super::AppState;

const ON_BEHALF_OF_HEADER_NAME: &str = "X-On-Behalf-Of";

// Partial state from the main app state to facilitate writing tests for the
// middleware.
#[derive(Clone)]
pub struct AuthenticationState {
    authenticator: Authenticator,
    config: super::config::ApiSvcConfig,
}

impl FromRef<Arc<AppState>> for AuthenticationState {
    fn from_ref(input: &Arc<AppState>) -> Self {
        Self {
            authenticator: input.authenticator.clone(),
            config: input.context.service_config(),
        }
    }
}

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
    state: &AuthenticationState,
    req: &Request<B>,
) -> Result<AuthenticationStatus, ApiError> {
    let auth_key = get_auth_key(req.headers())?;
    let Some(auth_key) = auth_key else {
        return Ok(AuthenticationStatus::Unauthenticated);
    };
    let config = &state.config;
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

    let project = state
        .authenticator
        .authenticate(&user_provided_secret)
        .await;
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
    State(state): State<AuthenticationState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_status = get_auth_status(&state, &req).await?;

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

#[cfg(test)]
mod tests {

    use std::collections::HashSet;
    use std::fmt::Debug;

    use axum::routing::get;
    use axum::{middleware, Router};
    use cronback_api_model::admin::CreateAPIkeyRequest;
    use hyper::{Body, StatusCode};
    use tower::ServiceExt;

    use super::*;
    use crate::api::auth_store::AuthStore;
    use crate::api::config::ApiSvcConfig;
    use crate::api::ApiService;

    async fn make_state() -> AuthenticationState {
        let mut set = HashSet::new();
        set.insert("adminkey1".to_string());
        set.insert("adminkey2".to_string());

        let config = ApiSvcConfig {
            address: String::new(),
            port: 123,
            database_uri: String::new(),
            admin_api_keys: set,
            log_request_body: false,
            log_response_body: false,
        };

        let db = ApiService::in_memory_database().await.unwrap();
        let auth_store = AuthStore::new(db);
        let authenticator = Authenticator::new(auth_store);

        AuthenticationState {
            authenticator,
            config,
        }
    }

    struct TestInput {
        app: Router,
        auth_header: Option<String>,
        on_behalf_on_header: Option<String>,
        expected_status: StatusCode,
    }

    impl Debug for TestInput {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TestInput")
                .field("auth_header", &self.auth_header)
                .field("on_behalf_on_header", &self.on_behalf_on_header)
                .field("expected_status", &self.expected_status)
                .finish()
        }
    }

    struct TestExpectations {
        unauthenticated: StatusCode,
        authenticated: StatusCode,
        admin_no_project: StatusCode,
        admin_with_project: StatusCode,
        unknown_secret_key: StatusCode,
    }

    async fn run_tests(
        app: Router,
        state: AuthenticationState,
        expectations: TestExpectations,
    ) -> anyhow::Result<()> {
        // Define one project and generate a key for it.
        let prj1 = ProjectId::generate();
        let key = state
            .authenticator
            .gen_key(
                CreateAPIkeyRequest {
                    key_name: "test".to_string(),
                    metadata: Default::default(),
                },
                &prj1,
            )
            .await?;

        let inputs = vec![
            // Unauthenticated user
            TestInput {
                app: app.clone(),
                auth_header: None,
                on_behalf_on_header: None,
                expected_status: expectations.unauthenticated,
            },
            // Authenticated user
            TestInput {
                app: app.clone(),
                auth_header: Some(format!("Bearer {}", key.unsafe_to_string())),
                on_behalf_on_header: None,
                expected_status: expectations.authenticated,
            },
            // Admin without project
            TestInput {
                app: app.clone(),
                auth_header: Some("Bearer adminkey1".to_string()),
                on_behalf_on_header: None,
                expected_status: expectations.admin_no_project,
            },
            // Admin with project
            TestInput {
                app: app.clone(),
                auth_header: Some("Bearer adminkey1".to_string()),
                on_behalf_on_header: Some(prj1.to_string()),
                expected_status: expectations.admin_with_project,
            },
            // Unknown secret key
            TestInput {
                app: app.clone(),
                auth_header: Some(format!(
                    "Bearer {}",
                    SecretApiKey::generate().unsafe_to_string()
                )),
                on_behalf_on_header: Some(prj1.to_string()),
                expected_status: expectations.unknown_secret_key,
            },
            // Malformed secret key should be treated as an unknown secret key
            TestInput {
                app: app.clone(),
                auth_header: Some("Bearer wrong key".to_string()),
                on_behalf_on_header: Some("wrong_project".to_string()),
                expected_status: expectations.unknown_secret_key,
            },
            // Malformed authorization header
            TestInput {
                app: app.clone(),
                auth_header: Some(format!("Token {}", key.unsafe_to_string())),
                on_behalf_on_header: Some(prj1.to_string()),
                expected_status: StatusCode::BAD_REQUEST,
            },
            // Malformed on-behalf-on project id
            TestInput {
                app: app.clone(),
                auth_header: Some("Bearer adminkey1".to_string()),
                on_behalf_on_header: Some("wrong_project".to_string()),
                expected_status: StatusCode::BAD_REQUEST,
            },
        ];

        for input in inputs {
            let input_str = format!("{:?}", input);

            let mut req = Request::builder();
            if let Some(v) = input.auth_header {
                req = req.header("Authorization", v);
            }
            if let Some(v) = input.on_behalf_on_header {
                req = req.header(ON_BEHALF_OF_HEADER_NAME, v);
            }

            let resp = input
                .app
                .oneshot(req.uri("/").body(Body::empty()).unwrap())
                .await?;

            assert_eq!(
                resp.status(),
                input.expected_status,
                "Input: {}",
                input_str
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_ensure_authenticated() -> anyhow::Result<()> {
        let state = make_state().await;

        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .layer(middleware::from_fn(super::ensure_authenticated))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                super::authenticate,
            ));

        run_tests(
            app,
            state,
            TestExpectations {
                unauthenticated: StatusCode::UNAUTHORIZED,
                authenticated: StatusCode::OK,
                admin_no_project: StatusCode::BAD_REQUEST,
                admin_with_project: StatusCode::OK,
                unknown_secret_key: StatusCode::UNAUTHORIZED,
            },
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_ensure_admin() -> anyhow::Result<()> {
        let state = make_state().await;

        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .layer(middleware::from_fn(super::ensure_admin))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                super::authenticate,
            ));

        run_tests(
            app,
            state,
            TestExpectations {
                unauthenticated: StatusCode::UNAUTHORIZED,
                authenticated: StatusCode::FORBIDDEN,
                admin_no_project: StatusCode::OK,
                admin_with_project: StatusCode::OK,
                unknown_secret_key: StatusCode::UNAUTHORIZED,
            },
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_ensure_admin_for_project() -> anyhow::Result<()> {
        let state = make_state().await;

        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .layer(middleware::from_fn(super::ensure_admin_for_project))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                super::authenticate,
            ));

        run_tests(
            app,
            state,
            TestExpectations {
                unauthenticated: StatusCode::UNAUTHORIZED,
                authenticated: StatusCode::FORBIDDEN,
                admin_no_project: StatusCode::BAD_REQUEST,
                admin_with_project: StatusCode::OK,
                unknown_secret_key: StatusCode::UNAUTHORIZED,
            },
        )
        .await?;

        Ok(())
    }
}
