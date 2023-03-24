use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{self, Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use base64::Engine;
use sha2::{Digest, Sha512};
use tracing::error;
use uuid::Uuid;

use crate::auth_store::AuthStoreError;
use crate::AppState;

#[derive(Default)]
pub enum HashVersion {
    #[default]
    V1,
}

impl Display for HashVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | HashVersion::V1 => write!(f, "v1"),
        }
    }
}

impl FromStr for HashVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            | "v1" => Ok(HashVersion::V1),
            | v => Err(format!("Invalid version: {v}")),
        }
    }
}

pub struct HashedApiKey {
    pub key_id: String,
    pub hash: String,
    pub hash_version: HashVersion,
}

// To avoid leaking the plaintext key anywhere, this struct doesn't allow you
// to unwrap the inner plaintext key and doesn't implement Debug/Display
#[cfg_attr(test, derive(Clone))]
pub struct ApiKey {
    key_id: String,
    plain_secret: String,
}

impl FromStr for ApiKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(s) = s.strip_prefix("sk_") else {
            return Err("API key doesn't start with sk_".to_string())
        };

        match s.split_once('_') {
            | Some((id, secret)) => {
                Ok(Self {
                    key_id: id.to_string(),
                    plain_secret: secret.to_string(),
                })
            }
            | None => Err("Malformed API key".to_string()),
        }
    }
}

impl ApiKey {
    pub fn generate() -> Self {
        Self {
            key_id: Uuid::new_v4().simple().to_string(),
            plain_secret: Uuid::new_v4().simple().to_string(),
        }
    }

    pub fn hash(&self, version: HashVersion) -> HashedApiKey {
        match version {
            | HashVersion::V1 => {
                let hash =
                    Sha512::digest(&self.plain_secret).as_slice().to_vec();
                HashedApiKey {
                    key_id: self.key_id.clone(),
                    hash: base64::engine::general_purpose::STANDARD
                        .encode(hash),
                    hash_version: HashVersion::V1,
                }
            }
        }
    }

    pub fn key_id(&self) -> &String {
        &self.key_id
    }

    pub fn unsafe_to_string(&self) -> String {
        format!("sk_{}_{}", self.key_id, self.plain_secret)
    }
}

pub async fn auth<B>(
    State(state): State<Arc<AppState>>,
    mut req: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if auth_header.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let auth_key = match auth_header.split_once(' ') {
        | Some((name, content)) if name == "Bearer" => content,
        | _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let Ok(api_key) = auth_key.to_string().parse() else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    match state.db.auth_store.validate_key(&api_key).await {
        | Ok(owner_id) => {
            req.extensions_mut().insert(owner_id);
            Ok(next.run(req).await)
        }
        | Err(AuthStoreError::AuthFailed(_)) => Err(StatusCode::UNAUTHORIZED),
        | Err(e) => {
            error!("Failed to authenticate user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ApiKey;

    #[test]
    fn test_api_key() {
        let api_key = ApiKey {
            key_id: "key1".to_string(),
            plain_secret: "supersecure".to_string(),
        };

        let serialized = api_key.unsafe_to_string();

        assert_eq!(serialized, "sk_key1_supersecure");

        let parsed_api_key: ApiKey = serialized.parse().unwrap();

        assert_eq!(api_key.key_id, parsed_api_key.key_id);
        assert_eq!(api_key.plain_secret, parsed_api_key.plain_secret);
    }
}
