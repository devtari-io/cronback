use async_trait::async_trait;
use sea_query::{ColumnDef, Iden, Table};

use super::errors::DatabaseError;
use super::helpers::{get_by_id_query, insert_query, paginated_query, KVIden};
use crate::database::Database;
use crate::types::{AttemptLogId, EmitAttemptLog, InvocationId, ShardedId};

#[derive(Iden)]
enum AttemptsIden {
    Attempts,
}

pub type AttemptLogStoreError = DatabaseError;

#[async_trait]
pub trait AttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: &EmitAttemptLog,
    ) -> Result<(), AttemptLogStoreError>;

    async fn get_attempts_for_invocation(
        &self,
        id: &InvocationId,
        before: Option<AttemptLogId>,
        after: Option<AttemptLogId>,
        limit: usize,
    ) -> Result<Vec<EmitAttemptLog>, AttemptLogStoreError>;

    async fn get_attempt(
        &self,
        id: &AttemptLogId,
    ) -> Result<Option<EmitAttemptLog>, AttemptLogStoreError>;
}

pub struct SqlAttemptLogStore {
    db: Database,
}

impl SqlAttemptLogStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn prepare(&self) -> Result<(), AttemptLogStoreError> {
        let sql = Table::create()
            .table(AttemptsIden::Attempts)
            .if_not_exists()
            .col(ColumnDef::new(KVIden::Id).text().primary_key())
            .col(ColumnDef::new(KVIden::Value).json_binary())
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl AttemptLogStore for SqlAttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: &EmitAttemptLog,
    ) -> Result<(), AttemptLogStoreError> {
        insert_query(&self.db, AttemptsIden::Attempts, &attempt.id, attempt)
            .await
    }

    async fn get_attempts_for_invocation(
        &self,
        id: &InvocationId,
        before: Option<AttemptLogId>,
        after: Option<AttemptLogId>,
        limit: usize,
    ) -> Result<Vec<EmitAttemptLog>, AttemptLogStoreError> {
        paginated_query(
            &self.db,
            AttemptsIden::Attempts,
            "invocation",
            id.value(),
            &before,
            &after,
            limit,
        )
        .await
    }

    async fn get_attempt(
        &self,
        id: &AttemptLogId,
    ) -> Result<Option<EmitAttemptLog>, AttemptLogStoreError> {
        get_by_id_query(&self.db, AttemptsIden::Attempts, id).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use chrono::{Timelike, Utc};
    use chrono_tz::UTC;

    use super::{AttemptLogStore, SqlAttemptLogStore};
    use crate::database::Database;
    use crate::types::{
        AttemptLogId,
        EmitAttemptLog,
        InvocationId,
        Payload,
        ProjectId,
        TriggerId,
        WebhookAttemptDetails,
    };

    fn build_attempt(invocation_id: &InvocationId) -> EmitAttemptLog {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_timezone(&UTC).with_nanosecond(0).unwrap();

        let project = ProjectId::new();
        EmitAttemptLog {
            id: AttemptLogId::new(&project),
            invocation: invocation_id.clone(),
            trigger: TriggerId::new(&project),
            project: project.clone(),
            status: crate::types::AttemptStatus::Succeeded,
            details: crate::types::AttemptDetails::WebhookAttemptDetails(
                WebhookAttemptDetails {
                    response_code: Some(404),
                    response_latency_s: Duration::from_secs(10),
                    response_payload: Some(Payload {
                        headers: HashMap::new(),
                        content_type: "application/json".to_string(),
                        body: "body".to_string(),
                    }),
                    error_message: None,
                },
            ),
            created_at: now,
        }
    }

    #[tokio::test]
    async fn test_sql_trigger_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlAttemptLogStore::new(db);
        store.prepare().await?;

        let owner = ProjectId::new();
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
        let mut results = store
            .get_attempts_for_invocation(&inv1, None, None, 100)
            .await?;
        let mut expected = vec![a1, a3];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(results, expected);

        Ok(())
    }
}
