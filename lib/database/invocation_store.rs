use async_trait::async_trait;
use sea_query::{ColumnDef, Iden, Table};

use super::errors::DatabaseError;
use super::helpers::{get_by_id_query, insert_query, paginated_query, KVIden};
use crate::database::Database;
use crate::types::{Invocation, InvocationId, ProjectId, ShardedId, TriggerId};

#[derive(Iden)]
enum InvocationsIden {
    Invocations,
}

pub type InvocationStoreError = DatabaseError;

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

    async fn get_invocations_by_trigger(
        &self,
        trigger_id: &TriggerId,
        before: Option<InvocationId>,
        after: Option<InvocationId>,
        limit: usize,
    ) -> Result<Vec<Invocation>, InvocationStoreError>;

    async fn get_invocations_by_owner(
        &self,
        owner_id: &ProjectId,
        before: Option<InvocationId>,
        after: Option<InvocationId>,
        limit: usize,
    ) -> Result<Vec<Invocation>, InvocationStoreError>;
}

pub struct SqlInvocationStore {
    db: Database,
}

impl SqlInvocationStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn prepare(&self) -> Result<(), InvocationStoreError> {
        let sql = Table::create()
            .table(InvocationsIden::Invocations)
            .if_not_exists()
            .col(ColumnDef::new(KVIden::Id).text().primary_key())
            .col(ColumnDef::new(KVIden::Value).json_binary())
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl InvocationStore for SqlInvocationStore {
    async fn store_invocation(
        &self,
        invocation: &Invocation,
    ) -> Result<(), InvocationStoreError> {
        insert_query(
            &self.db,
            InvocationsIden::Invocations,
            &invocation.id,
            invocation,
        )
        .await
    }

    async fn get_invocation(
        &self,
        id: &InvocationId,
    ) -> Result<Option<Invocation>, InvocationStoreError> {
        get_by_id_query(&self.db, InvocationsIden::Invocations, id).await
    }

    async fn get_invocations_by_trigger(
        &self,
        trigger_id: &TriggerId,
        before: Option<InvocationId>,
        after: Option<InvocationId>,
        limit: usize,
    ) -> Result<Vec<Invocation>, InvocationStoreError> {
        paginated_query(
            &self.db,
            InvocationsIden::Invocations,
            "trigger",
            trigger_id.value(),
            &before,
            &after,
            limit,
        )
        .await
    }

    async fn get_invocations_by_owner(
        &self,
        project: &ProjectId,
        before: Option<InvocationId>,
        after: Option<InvocationId>,
        limit: usize,
    ) -> Result<Vec<Invocation>, InvocationStoreError> {
        paginated_query(
            &self.db,
            InvocationsIden::Invocations,
            "project",
            project.value(),
            &before,
            &after,
            limit,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Timelike, Utc};
    use chrono_tz::UTC;

    use super::{InvocationStore, SqlInvocationStore};
    use crate::database::Database;
    use crate::types::{
        Invocation,
        InvocationId,
        InvocationStatus,
        ProjectId,
        TriggerId,
        Webhook,
        WebhookDeliveryStatus,
        WebhookStatus,
    };

    fn build_invocation(
        trigger_id: TriggerId,
        project: ProjectId,
    ) -> Invocation {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_timezone(&UTC).with_nanosecond(0).unwrap();

        Invocation {
            id: InvocationId::new(&project),
            trigger: trigger_id,
            project,
            created_at: now,
            status: vec![InvocationStatus::WebhookStatus(WebhookStatus {
                webhook: Webhook {
                    _kind: Default::default(),
                    url: Some("http://test".to_string()),
                    http_method: crate::types::HttpMethod::GET,
                    timeout_s: Duration::from_secs(5),
                    retry: None,
                },
                delivery_status: WebhookDeliveryStatus::Attempting,
            })],
            payload: None,
        }
    }

    #[tokio::test]
    async fn test_sql_invocation_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlInvocationStore::new(db);
        store.prepare().await?;

        let owner1 = ProjectId::new();
        let owner2 = ProjectId::new();
        let t1 = TriggerId::new(&owner1);
        let t2 = TriggerId::new(&owner2);

        let mut i1 = build_invocation(t1.clone(), owner1.clone());
        let i2 = build_invocation(t2.clone(), owner2.clone());
        let i3 = build_invocation(t1.clone(), owner1.clone());

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

        // Test get invocations by trigger
        let mut results = store
            .get_invocations_by_trigger(&t1, None, None, 100)
            .await?;
        let mut expected = vec![i1.clone(), i3.clone()];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results, expected);

        // Test get invocations by owner
        let results = store
            .get_invocations_by_owner(&owner2, None, None, 100)
            .await?;
        let expected = vec![i2.clone()];
        assert_eq!(results, expected);

        i1.status = vec![InvocationStatus::WebhookStatus(WebhookStatus {
            webhook: Webhook {
                _kind: Default::default(),
                url: Some("http://test".to_string()),
                http_method: crate::types::HttpMethod::GET,
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
