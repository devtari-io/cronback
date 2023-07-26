use std::fmt::Display;
use std::str::FromStr;

use base64::Engine;
use chrono::Utc;
use cronback_api_model::admin::CreateAPIkeyRequest;
use lib::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha512};
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

use super::auth_store::AuthStore;
use super::db_model::{api_keys, ApiKey};
use super::errors::ApiError;

pub static API_KEY_PREFIX: &str = "sk_";

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("auth failed: {0}")]
    AuthFailed(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<AuthError> for ApiError {
    fn from(value: AuthError) -> Self {
        match value {
            | AuthError::Database(e) => {
                error!("{}", e);
                ApiError::ServiceUnavailable
            }
            | AuthError::Internal(e) => {
                error!("{}", e);
                ApiError::ServiceUnavailable
            }
            | AuthError::AuthFailed(_) => ApiError::Unauthorized,
        }
    }
}

pub struct Authenticator {
    store: AuthStore,
}

impl Authenticator {
    pub fn new(store: AuthStore) -> Self {
        Self { store }
    }

    pub async fn gen_key(
        &self,
        req: CreateAPIkeyRequest,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<SecretApiKey, AuthError> {
        let key = SecretApiKey::generate();
        let hashed = key.hash(HashVersion::default());

        let model = ApiKey {
            key_id: hashed.key_id,
            hash: hashed.hash,
            hash_version: hashed.hash_version.to_string(),
            project_id: project.clone(),
            name: req.key_name,
            created_at: Utc::now(),
            metadata: api_keys::Metadata {
                creator_user_id: req.metadata.creator_user_id,
            },
        };

        self.store.save_key(model).await?;

        Ok(key)
    }

    pub async fn authenticate(
        &self,
        user_provided_secret: &SecretApiKey,
    ) -> Result<ValidShardedId<ProjectId>, AuthError> {
        let key_model =
            self.store.get_key(user_provided_secret.key_id()).await?;

        let Some(key_model) = key_model else {
            // key_id doesn't exist in the database
            return Err(AuthError::AuthFailed(
                "key_id doesn't exist".to_string(),
            ));
        };

        let hash_version = key_model.hash_version;
        let hash_version: HashVersion = hash_version.parse().map_err(|_| {
            AuthError::Internal(format!("Unknown version: {hash_version}"))
        })?;

        let user_provided_hash = user_provided_secret.hash(hash_version);
        let stored_hash = key_model.hash;

        if user_provided_hash.hash != stored_hash {
            return Err(AuthError::AuthFailed(
                "Mismatched secret key".to_string(),
            ));
        }

        Ok(key_model.project_id)
    }

    pub async fn revoke_key(
        &self,
        key_id: &str,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<bool, AuthError> {
        let res = self.store.delete_key(key_id, project).await?;
        Ok(res)
    }

    pub async fn list_keys(
        &self,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<Vec<ApiKey>, AuthError> {
        let res = self.store.list_keys(project).await?;
        Ok(res)
    }
}

#[derive(Default)]
enum HashVersion {
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

struct HashedApiKey {
    pub key_id: String,
    pub hash: String,
    pub hash_version: HashVersion,
}

// To avoid leaking the plaintext key anywhere, this struct doesn't allow you
// to unwrap the inner plaintext key and doesn't implement Debug/Display
#[cfg_attr(test, derive(Clone))]
pub struct SecretApiKey {
    key_id: String,
    plain_secret: String,
}

impl FromStr for SecretApiKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(s) = s.strip_prefix(API_KEY_PREFIX) else {
            return Err(format!(
                "API key doesn't start with {}",
                API_KEY_PREFIX
            ));
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

impl SecretApiKey {
    fn generate() -> Self {
        Self {
            key_id: Uuid::new_v4().simple().to_string(),
            plain_secret: Uuid::new_v4().simple().to_string(),
        }
    }

    fn hash(&self, version: HashVersion) -> HashedApiKey {
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
        format!("{}{}_{}", API_KEY_PREFIX, self.key_id, self.plain_secret)
    }

    /// Returns true if the passed string potentially contains a secret key
    pub fn matches(string: &str) -> bool {
        static REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"sk_[0-9a-f]{32}_[0-9a-f]{32}").unwrap());
        REGEX.is_match(string)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cronback_api_model::admin::CreateAPIkeyRequest;

    use super::*;
    use crate::api::ApiService;

    #[test]
    fn test_api_key() {
        let api_key = SecretApiKey {
            key_id: "key1".to_string(),
            plain_secret: "supersecure".to_string(),
        };

        let serialized = api_key.unsafe_to_string();

        assert_eq!(serialized, "sk_key1_supersecure");

        let parsed_api_key: SecretApiKey = serialized.parse().unwrap();

        assert_eq!(api_key.key_id, parsed_api_key.key_id);
        assert_eq!(api_key.plain_secret, parsed_api_key.plain_secret);
    }

    fn build_create_key_request(name: &str) -> CreateAPIkeyRequest {
        CreateAPIkeyRequest {
            key_name: name.to_string(),
            metadata: cronback_api_model::admin::APIKeyMetaData {
                creator_user_id: None,
            },
        }
    }

    #[tokio::test]
    async fn test_auth_store() -> anyhow::Result<()> {
        let db = ApiService::in_memory_database().await?;
        let store = AuthStore::new(db);

        let prj1 = ProjectId::generate();
        let prj2 = ProjectId::generate();

        let authenticator = Authenticator::new(store);

        let key1 = authenticator
            .gen_key(build_create_key_request("key1"), &prj1)
            .await?;
        let key2 = authenticator
            .gen_key(build_create_key_request("key2"), &prj2)
            .await?;
        let key3 = authenticator
            .gen_key(build_create_key_request("key3"), &prj1)
            .await?;
        let key4 = authenticator
            .gen_key(build_create_key_request("key4"), &prj2)
            .await?;

        // Test authenticate
        assert_eq!(prj1, authenticator.authenticate(&key1).await?);
        assert_eq!(prj2, authenticator.authenticate(&key2).await?);
        assert_eq!(prj1, authenticator.authenticate(&key3).await?);
        assert_eq!(prj2, authenticator.authenticate(&key4).await?);

        // Unknown key id
        let key5 = SecretApiKey::from_str("sk_notfound_secret4").unwrap();
        assert!(matches!(
            authenticator.authenticate(&key5).await,
            Err(AuthError::AuthFailed(_))
        ));

        // Wrong secret
        let key5 = SecretApiKey::from_str("sk_key1_wrongsecret").unwrap();
        assert!(matches!(
            authenticator.authenticate(&key5).await,
            Err(AuthError::AuthFailed(_))
        ));

        // Test delete key
        assert!(authenticator.revoke_key(key1.key_id(), &prj1).await?);
        assert!(matches!(
            authenticator.authenticate(&key1).await,
            Err(AuthError::AuthFailed(_))
        ));

        // Test List keys
        assert_eq!(
            authenticator
                .list_keys(&prj2)
                .await?
                .into_iter()
                .map(|k| k.name)
                .collect::<Vec<_>>(),
            vec!["key2".to_string(), "key4".to_string()]
        );

        Ok(())
    }

    #[test]
    fn test_secret_api_key_regex_matching() {
        assert!(SecretApiKey::matches(
            SecretApiKey::generate().unsafe_to_string().as_ref()
        ));
        assert!(SecretApiKey::matches(
            SecretApiKey::generate().unsafe_to_string().as_ref()
        ));
        assert!(!SecretApiKey::matches("sk_key1_plain"));
    }
}
