use async_trait::async_trait;
use lib::database::models::api_keys;
use lib::database::models::prelude::ApiKeys;
use lib::database::Database;
use lib::model::{ModelId, ValidShardedId};
use lib::types::ProjectId;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use thiserror::Error;

use crate::auth::{ApiKey, HashVersion};

#[derive(Error, Debug)]
pub enum AuthStoreError {
    #[error("database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error("auth failed: {0}")]
    AuthFailed(String),
    #[error("internal error: {0}")]
    InternalError(String),
}

#[async_trait]
pub trait AuthStore {
    async fn save_key(
        &self,
        key: ApiKey,
        project: &ValidShardedId<ProjectId>,
        key_name: &str,
    ) -> Result<(), AuthStoreError>;

    async fn validate_key(
        &self,
        key: &ApiKey,
    ) -> Result<ValidShardedId<ProjectId>, AuthStoreError>;

    /// Returns true if the key got deleted, false if the key didn't exist
    async fn delete_key(
        &self,
        key_id: String,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<bool, AuthStoreError>;

    async fn list_keys(
        &self,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<Vec<api_keys::Model>, AuthStoreError>;
}

pub struct SqlAuthStore {
    db: Database,
}

impl SqlAuthStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AuthStore for SqlAuthStore {
    async fn save_key(
        &self,
        key: ApiKey,
        project_id: &ValidShardedId<ProjectId>,
        key_name: &str,
    ) -> Result<(), AuthStoreError> {
        let hashed = key.hash(HashVersion::default());

        let model = api_keys::ActiveModel {
            key_id: sea_orm::ActiveValue::Set(hashed.key_id),
            hash: sea_orm::ActiveValue::Set(hashed.hash),
            hash_version: sea_orm::ActiveValue::Set(
                hashed.hash_version.to_string(),
            ),
            project_id: sea_orm::ActiveValue::Set(project_id.clone()),
            name: sea_orm::ActiveValue::Set(key_name.to_string()),
        };

        api_keys::Entity::insert(model).exec(&self.db.orm).await?;
        Ok(())
    }

    async fn validate_key(
        &self,
        user_provided_key: &ApiKey,
    ) -> Result<ValidShardedId<ProjectId>, AuthStoreError> {
        let result = ApiKeys::find_by_id(user_provided_key.key_id())
            .one(&self.db.orm)
            .await?;

        let Some(result) = result else {
            return Err(AuthStoreError::AuthFailed(
                "key_id not found".to_string(),
            ));
        };

        let hash_version = result.hash_version;
        let hash_version: HashVersion = hash_version.parse().map_err(|_| {
            AuthStoreError::InternalError(format!(
                "Unknown version: {hash_version}"
            ))
        })?;

        let user_provided_hash = user_provided_key.hash(hash_version);
        let stored_hash = result.hash;

        if user_provided_hash.hash != stored_hash {
            return Err(AuthStoreError::AuthFailed(
                "Mismatched secret key".to_string(),
            ));
        }

        Ok(ProjectId::from(result.project_id)
            .validated()
            .expect("Invalid ProjectId persisted in database"))
    }

    async fn delete_key(
        &self,
        key_id: String,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<bool, AuthStoreError> {
        let res = ApiKeys::delete_many()
            .filter(api_keys::Column::KeyId.eq(key_id))
            .filter(api_keys::Column::ProjectId.eq(project.clone()))
            .exec(&self.db.orm)
            .await?;
        Ok(res.rows_affected > 0)
    }

    async fn list_keys(
        &self,
        project: &ValidShardedId<ProjectId>,
    ) -> Result<Vec<api_keys::Model>, AuthStoreError> {
        let results = ApiKeys::find()
            .filter(api_keys::Column::ProjectId.eq(project.clone()))
            .all(&self.db.orm)
            .await?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use lib::database::Database;
    use lib::types::ProjectId;

    use super::{AuthStore, AuthStoreError, SqlAuthStore};
    use crate::auth::ApiKey;

    #[tokio::test]
    async fn test_sql_auth_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlAuthStore::new(db);

        let owner1 = ProjectId::generate();
        let owner2 = ProjectId::generate();

        let key1 = ApiKey::from_str("sk_key1_secret1").unwrap();
        let key2 = ApiKey::from_str("sk_key2_secret2").unwrap();
        let key3 = ApiKey::from_str("sk_key3_secret3").unwrap();
        let key4 = ApiKey::from_str("sk_key4_secret4").unwrap();

        // Test save keys
        store.save_key(key1.clone(), &owner1, "key1").await?;
        store.save_key(key2.clone(), &owner2, "key2").await?;
        store.save_key(key3.clone(), &owner1, "key3").await?;
        store.save_key(key4.clone(), &owner2, "key4").await?;

        // Test find owner by key
        assert_eq!(owner1, store.validate_key(&key1).await?);
        assert_eq!(owner2, store.validate_key(&key2).await?);
        assert_eq!(owner1, store.validate_key(&key3).await?);
        assert_eq!(owner2, store.validate_key(&key4).await?);

        // Unknown key id
        let key5 = ApiKey::from_str("sk_notfound_secret4").unwrap();
        assert!(matches!(
            store.validate_key(&key5).await,
            Err(AuthStoreError::AuthFailed(_))
        ));

        // Wrong secret
        let key5 = ApiKey::from_str("sk_key1_wrongsecret").unwrap();
        assert!(matches!(
            store.validate_key(&key5).await,
            Err(AuthStoreError::AuthFailed(_))
        ));

        // Test delete key
        assert!(store.delete_key(key1.key_id().clone(), &owner1).await?);
        assert!(matches!(
            store.validate_key(&key1).await,
            Err(AuthStoreError::AuthFailed(_))
        ));

        // After deletion, other keys should continue to work
        assert_eq!(owner2, store.validate_key(&key2).await?);

        // Deleting an already deleted key should return false.
        assert!(!store.delete_key(key1.key_id().clone(), &owner1).await?);

        // Deleting a key with the wrong project returns false
        assert!(!store.delete_key(key4.key_id().clone(), &owner1).await?);
        assert_eq!(owner2, store.validate_key(&key4).await?);

        // Test List keys
        assert_eq!(
            store
                .list_keys(&owner1)
                .await?
                .into_iter()
                .map(|k| k.name)
                .collect::<Vec<_>>(),
            vec!["key3".to_string()]
        );
        assert_eq!(
            store
                .list_keys(&owner2)
                .await?
                .into_iter()
                .map(|k| k.name)
                .collect::<Vec<_>>(),
            vec!["key2".to_string(), "key4".to_string()]
        );

        Ok(())
    }
}
