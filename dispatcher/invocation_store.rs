use async_trait::async_trait;
use shared::database::SqliteDatabase;
use shared::types::{Invocation, InvocationId};
use sqlx::Row;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InvocationStoreError {
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    ParseError(#[from] serde_json::Error),
}

#[async_trait]
pub trait InvocationStore {
    async fn store_invocation(
        &self,
        invocation: &Invocation,
    ) -> Result<(), InvocationStoreError>;

    async fn get_invocation(
        &self,
        id: &InvocationId,
    ) -> Result<Option<Invocation>, InvocationStoreError>;
}

pub struct SqlInvocationStore {
    db: SqliteDatabase,
}

impl SqlInvocationStore {
    pub async fn create(
        db: SqliteDatabase,
    ) -> Result<Self, InvocationStoreError> {
        let s = Self { db };
        s.prepare().await?;
        Ok(s)
    }

    async fn prepare(&self) -> Result<(), InvocationStoreError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS invocations (
                id TEXT PRIMARY KEY,
                value TEXT
            )
        "#,
        )
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl InvocationStore for SqlInvocationStore {
    async fn store_invocation(
        &self,
        invocation: &Invocation,
    ) -> Result<(), InvocationStoreError> {
        sqlx::query(
            "INSERT OR REPLACE INTO invocations (id,value) VALUES (?,?)",
        )
        .bind(&invocation.id.to_string())
        .bind(serde_json::to_string(invocation)?)
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    async fn get_invocation(
        &self,
        id: &InvocationId,
    ) -> Result<Option<Invocation>, InvocationStoreError> {
        let result = sqlx::query("SELECT value FROM invocations WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.db.pool)
            .await;

        match result {
            | Ok(r) => {
                let j = r.get::<String, _>("value");
                Ok(Some(serde_json::from_str::<Invocation>(&j)?))
            }
            | Err(sqlx::Error::RowNotFound) => Ok(None),
            | Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Timelike, Utc};
    use chrono_tz::UTC;
    use shared::database::SqliteDatabase;
    use shared::types::{
        Invocation,
        InvocationId,
        InvocationStatus,
        OwnerId,
        Payload,
        TriggerId,
        Webhook,
        WebhookDeliveryStatus,
        WebhookStatus,
    };

    use super::{InvocationStore, SqlInvocationStore};

    fn build_invocation() -> Invocation {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_timezone(&UTC).with_nanosecond(0).unwrap();

        let owner = OwnerId::new();
        Invocation {
            id: InvocationId::new(&owner),
            trigger_id: TriggerId::new(&owner),
            owner_id: owner,
            created_at: now,
            status: vec![InvocationStatus::WebhookStatus(WebhookStatus {
                webhook: Webhook {
                    url: Some("http://test".to_string()),
                    http_method: shared::types::HttpMethod::GET,
                    timeout_s: Duration::from_secs(5),
                    retry: None,
                },
                delivery_status: WebhookDeliveryStatus::Attempting,
            })],
            payload: Payload::default(),
        }
    }

    #[tokio::test]
    async fn test_sql_invocation_store() -> anyhow::Result<()> {
        let db = SqliteDatabase::in_memory().await?;
        let store = SqlInvocationStore::create(db).await?;

        let mut i1 = build_invocation();
        let i2 = build_invocation();
        let i3 = build_invocation();

        // Test stores
        store.store_invocation(&i1).await?;
        store.store_invocation(&i2).await?;
        store.store_invocation(&i3).await?;

        // Test getters
        assert_eq!(store.get_invocation(&i1.id).await?, Some(i1.clone()));
        assert_eq!(store.get_invocation(&i2.id).await?, Some(i2.clone()));
        assert_eq!(store.get_invocation(&i3.id).await?, Some(i3.clone()));

        // Test fetching non existent invocation
        assert_eq!(
            store
                .get_invocation(&InvocationId::from("non_existent".to_string()))
                .await?,
            None
        );

        i1.status = vec![InvocationStatus::WebhookStatus(WebhookStatus {
            webhook: Webhook {
                url: Some("http://test".to_string()),
                http_method: shared::types::HttpMethod::GET,
                timeout_s: Duration::from_secs(5),
                retry: None,
            },
            delivery_status: WebhookDeliveryStatus::Failed,
        })];

        // Update the invocation
        store.store_invocation(&i1).await?;
        assert_eq!(store.get_invocation(&i1.id).await?, Some(i1.clone()));

        Ok(())
    }
}
