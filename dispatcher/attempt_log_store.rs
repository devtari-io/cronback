use async_trait::async_trait;
use shared::{
    database::SqliteDatabase,
    types::{AttemptLogId, EmitAttemptLog, InvocationId},
};
use sqlx::Row;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AttemptLogStoreError {
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    ParseError(#[from] serde_json::Error),
}

#[async_trait]
pub trait AttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: &EmitAttemptLog,
    ) -> Result<(), AttemptLogStoreError>;

    async fn get_attempts_for_invocation(
        &self,
        id: &InvocationId,
    ) -> Result<Vec<EmitAttemptLog>, AttemptLogStoreError>;

    async fn get_attempt(
        &self,
        id: &AttemptLogId,
    ) -> Result<Option<EmitAttemptLog>, AttemptLogStoreError>;
}

pub struct SqlAttemptLogStore {
    db: SqliteDatabase,
}

impl SqlAttemptLogStore {
    pub async fn create(
        db: SqliteDatabase,
    ) -> Result<Self, AttemptLogStoreError> {
        let s = Self { db };
        s.prepare().await?;
        Ok(s)
    }

    async fn prepare(&self) -> Result<(), AttemptLogStoreError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS attempts (
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
impl AttemptLogStore for SqlAttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: &EmitAttemptLog,
    ) -> Result<(), AttemptLogStoreError> {
        sqlx::query("INSERT OR REPLACE INTO attempts (id,value) VALUES (?,?)")
            .bind(&attempt.id.to_string())
            .bind(serde_json::to_string(attempt)?)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    async fn get_attempts_for_invocation(
        &self,
        id: &InvocationId,
    ) -> Result<Vec<EmitAttemptLog>, AttemptLogStoreError> {
        let results = sqlx::query("SELECT value FROM attempts where JSON_EXTRACT(value, '$.invocation_id') = ?")
            .bind(&id.to_string())
            .fetch_all(&self.db.pool)
            .await?
            .into_iter().map(|r| {
                let j = r.get::<String, _>("value");
                serde_json::from_str::<EmitAttemptLog>(&j)
            })
            .collect::<Result<Vec<_>, _>>();
        Ok(results?)
    }

    async fn get_attempt(
        &self,
        id: &AttemptLogId,
    ) -> Result<Option<EmitAttemptLog>, AttemptLogStoreError> {
        let result = sqlx::query("SELECT value FROM attempts WHERE id = ?")
            .bind(&id.to_string())
            .fetch_one(&self.db.pool)
            .await;

        match result {
            | Ok(r) => {
                let j = r.get::<String, _>("value");
                Ok(Some(serde_json::from_str::<EmitAttemptLog>(&j)?))
            }
            | Err(sqlx::Error::RowNotFound) => Ok(None),
            | Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use chrono::{Timelike, Utc};
    use chrono_tz::UTC;
    use shared::{
        database::SqliteDatabase,
        types::{
            AttemptLogId, EmitAttemptLog, InvocationId, OwnerId, Payload,
            TriggerId, WebhookAttemptDetails,
        },
    };

    use super::AttemptLogStore;
    use super::SqlAttemptLogStore;

    fn build_attempt(invocation_id: &InvocationId) -> EmitAttemptLog {
        // Serialization drops nanoseconds, so to let's zero it here for easier equality comparisons
        let now = Utc::now().with_timezone(&UTC).with_nanosecond(0).unwrap();

        let owner = OwnerId::new();
        EmitAttemptLog {
            id: AttemptLogId::new(&owner),
            invocation_id: invocation_id.clone(),
            trigger_id: TriggerId::new(&owner),
            owner_id: owner.clone(),
            status: shared::types::AttemptStatus::Succeeded,
            details: shared::types::AttemptDetails::WebhookAttemptDetails(
                WebhookAttemptDetails {
                    response_code: Some(404),
                    response_latency_s: Duration::from_secs(10),
                    response_payload: Some(Payload {
                        headers: HashMap::new(),
                        content_type: "application/json".to_string(),
                        body: "body".to_string(),
                    }),
                    error_msg: None,
                },
            ),
            created_at: now,
        }
    }

    #[tokio::test]
    async fn test_sql_trigger_store() -> anyhow::Result<()> {
        let db = SqliteDatabase::in_memory().await?;
        let store = SqlAttemptLogStore::create(db).await?;

        let owner = OwnerId::new();
        let inv1 = InvocationId::new(&owner);
        let inv2 = InvocationId::new(&owner);

        let a1 = build_attempt(&inv1);
        let a2 = build_attempt(&inv2);
        let a3 = build_attempt(&inv1);

        // Test log attempts
        store.log_attempt(&a1).await?;
        store.log_attempt(&a2).await?;
        store.log_attempt(&a3).await?;

        // Test getters
        assert_eq!(store.get_attempt(&a1.id).await?, Some(a1.clone()));
        assert_eq!(store.get_attempt(&a2.id).await?, Some(a2.clone()));
        assert_eq!(store.get_attempt(&a3.id).await?, Some(a3.clone()));

        // Test fetching non existent attempt
        assert_eq!(
            store
                .get_attempt(&AttemptLogId::from("non_existent".to_string()))
                .await?,
            None
        );

        // Test get all for a certain invocation
        let mut results = store.get_attempts_for_invocation(&inv1).await?;
        let mut expected = vec![a1, a3];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(results, expected);

        Ok(())
    }
}
