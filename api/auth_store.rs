use async_trait::async_trait;
use lib::database::Database;
use lib::types::ProjectId;
use sea_query::{ColumnDef, Expr, Iden, Query, Table};
use sea_query_binder::SqlxBinder;
use sqlx::Row;
use thiserror::Error;

use crate::auth::{ApiKey, HashVersion};

#[derive(Iden)]
enum ApiKeysIden {
    ApiKeys,
    KeyId,
    Hash,
    HashVersion,
    Project,
    Name,
}

#[derive(Error, Debug)]
pub enum AuthStoreError {
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
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
        project: &ProjectId,
        key_name: &str,
    ) -> Result<(), AuthStoreError>;

    async fn validate_key(
        &self,
        key: &ApiKey,
    ) -> Result<ProjectId, AuthStoreError>;

    /// Returns true if the key got revoked, false if the key didn't exist
    async fn revoke_key(&self, key: &ApiKey) -> Result<bool, AuthStoreError>;
}

pub struct SqlAuthStore {
    db: Database,
}

impl SqlAuthStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn prepare(&self) -> Result<(), AuthStoreError> {
        let sql = Table::create()
            .table(ApiKeysIden::ApiKeys)
            .if_not_exists()
            .col(ColumnDef::new(ApiKeysIden::KeyId).text().primary_key())
            .col(ColumnDef::new(ApiKeysIden::Hash).text())
            .col(ColumnDef::new(ApiKeysIden::HashVersion).text())
            .col(ColumnDef::new(ApiKeysIden::Project).text())
            .col(ColumnDef::new(ApiKeysIden::Name).text())
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl AuthStore for SqlAuthStore {
    async fn save_key(
        &self,
        key: ApiKey,
        project: &ProjectId,
        key_name: &str,
    ) -> Result<(), AuthStoreError> {
        let hashed = key.hash(HashVersion::default());

        let (sql, values) = Query::insert()
            .into_table(ApiKeysIden::ApiKeys)
            .columns([
                ApiKeysIden::KeyId,
                ApiKeysIden::Hash,
                ApiKeysIden::HashVersion,
                ApiKeysIden::Project,
                ApiKeysIden::Name,
            ])
            .values_panic([
                hashed.key_id.into(),
                hashed.hash.into(),
                hashed.hash_version.to_string().into(),
                project.to_string().into(),
                key_name.to_string().into(),
            ])
            .build_any_sqlx(self.db.builder().as_ref());

        sqlx::query_with(&sql, values)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    async fn validate_key(
        &self,
        user_provided_key: &ApiKey,
    ) -> Result<ProjectId, AuthStoreError> {
        let (sql, values) = Query::select()
            .columns([
                ApiKeysIden::KeyId,
                ApiKeysIden::Hash,
                ApiKeysIden::HashVersion,
                ApiKeysIden::Project,
            ])
            .from(ApiKeysIden::ApiKeys)
            .and_where(
                Expr::col(ApiKeysIden::KeyId).eq(user_provided_key.key_id()),
            )
            .build_any_sqlx(self.db.builder().as_ref());
        let result = sqlx::query_with(&sql, values)
            .fetch_one(&self.db.pool)
            .await;
        let row = match result {
            | Ok(r) => r,
            | Err(sqlx::Error::RowNotFound) => {
                return Err(AuthStoreError::AuthFailed(
                    "key_id not found".to_string(),
                ));
            }
            | Err(e) => return Err(e.into()),
        };

        let hash_version =
            row.get::<String, _>(ApiKeysIden::HashVersion.to_string().as_str());
        let hash_version: HashVersion = hash_version.parse().map_err(|_| {
            AuthStoreError::InternalError(format!(
                "Unknown version: {hash_version}"
            ))
        })?;

        let user_provided_hash = user_provided_key.hash(hash_version);
        let stored_hash =
            row.get::<String, _>(ApiKeysIden::Hash.to_string().as_str());

        if user_provided_hash.hash != stored_hash {
            return Err(AuthStoreError::AuthFailed(
                "Mismatched secret key".to_string(),
            ));
        }

        Ok(ProjectId::from(row.get::<String, _>(
            ApiKeysIden::Project.to_string().as_str(),
        )))
    }

    async fn revoke_key(&self, key: &ApiKey) -> Result<bool, AuthStoreError> {
        let (sql, values) = Query::delete()
            .from_table(ApiKeysIden::ApiKeys)
            .and_where(Expr::col(ApiKeysIden::KeyId).eq(key.key_id()))
            .build_any_sqlx(self.db.builder().as_ref());
        let res = sqlx::query_with(&sql, values)
            .execute(&self.db.pool)
            .await?;
        Ok(res.rows_affected() > 0)
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
        store.prepare().await?;

        let owner1 = ProjectId::new();
        let owner2 = ProjectId::new();

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

        // Test revoke key
        assert!(store.revoke_key(&key1).await?);
        assert!(matches!(
            store.validate_key(&key1).await,
            Err(AuthStoreError::AuthFailed(_))
        ));

        // After revocation, other keys should continue to work
        assert_eq!(owner2, store.validate_key(&key2).await?);

        // Re-revoking an already revoked key should return false.
        assert!(!store.revoke_key(&key1).await?);

        Ok(())
    }
}
