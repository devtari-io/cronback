use async_trait::async_trait;
use sea_query::{ColumnDef, Expr, Iden, Index, Table};

use super::errors::DatabaseError;
use super::helpers::{
    get_by_id_query,
    insert_query,
    paginated_query,
    update_query,
    GeneratedJsonField,
    KVIden,
};
use crate::database::Database;
use crate::model::ModelId;
use crate::types::{Invocation, InvocationId, ProjectId, TriggerId};

#[derive(Iden)]
enum InvocationsIden {
    Invocations,
    TriggerId,
}

pub type InvocationStoreError = DatabaseError;

#[async_trait]
pub trait InvocationStore {
    async fn store_invocation(
        &self,
        invocation: &Invocation,
    ) -> Result<(), InvocationStoreError>;

    async fn update_invocation(
        &self,
        invocation: &Invocation,
    ) -> Result<(), InvocationStoreError>;

    async fn get_invocation(
        &self,
        project: &ProjectId,
        id: &InvocationId,
    ) -> Result<Option<Invocation>, InvocationStoreError>;

    async fn get_invocations_by_trigger(
        &self,
        project: &ProjectId,
        trigger_id: &TriggerId,
        before: Option<InvocationId>,
        after: Option<InvocationId>,
        limit: usize,
    ) -> Result<Vec<Invocation>, InvocationStoreError>;

    async fn get_invocations_by_project(
        &self,
        project: &ProjectId,
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
            .col(
                ColumnDef::new(KVIden::Project)
                    .text()
                    .generate_from_json_field(KVIden::Value, "project"),
            )
            .col(
                ColumnDef::new(InvocationsIden::TriggerId)
                    .text()
                    .generate_from_json_field(KVIden::Value, "trigger"),
            )
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;

        // Create the indicies
        let sql = Index::create()
            .if_not_exists()
            .name("IX_invocations_project")
            .table(InvocationsIden::Invocations)
            .col(KVIden::Project)
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;

        let sql = Index::create()
            .if_not_exists()
            .name("IX_invocations_triggerid")
            .table(InvocationsIden::Invocations)
            .col(InvocationsIden::TriggerId)
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

    async fn update_invocation(
        &self,
        invocation: &Invocation,
    ) -> Result<(), InvocationStoreError> {
        update_query(
            &self.db,
            InvocationsIden::Invocations,
            &invocation.project,
            &invocation.id,
            invocation,
        )
        .await
    }

    async fn get_invocation(
        &self,
        project: &ProjectId,
        id: &InvocationId,
    ) -> Result<Option<Invocation>, InvocationStoreError> {
        get_by_id_query(&self.db, InvocationsIden::Invocations, project, id)
            .await
    }

    async fn get_invocations_by_trigger(
        &self,
        project: &ProjectId,
        trigger_id: &TriggerId,
        before: Option<InvocationId>,
        after: Option<InvocationId>,
        limit: usize,
    ) -> Result<Vec<Invocation>, InvocationStoreError> {
        paginated_query(
            &self.db,
            InvocationsIden::Invocations,
            Expr::col(InvocationsIden::TriggerId)
                .eq(trigger_id.value())
                .and(Expr::col(KVIden::Project).eq(project.value())),
            &before,
            &after,
            Some(limit),
        )
        .await
    }

    async fn get_invocations_by_project(
        &self,
        project: &ProjectId,
        before: Option<InvocationId>,
        after: Option<InvocationId>,
        limit: usize,
    ) -> Result<Vec<Invocation>, InvocationStoreError> {
        paginated_query(
            &self.db,
            InvocationsIden::Invocations,
            Expr::col(KVIden::Project).eq(project.value()),
            &before,
            &after,
            Some(limit),
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
    use crate::database::errors::DatabaseError;
    use crate::database::Database;
    use crate::model::ValidShardedId;
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
        trigger_id: ValidShardedId<TriggerId>,
        project: ValidShardedId<ProjectId>,
    ) -> Invocation {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_timezone(&UTC).with_nanosecond(0).unwrap();

        Invocation {
            id: InvocationId::generate(&project).into(),
            trigger: trigger_id.into(),
            project,
            created_at: now,
            status: vec![InvocationStatus::WebhookStatus(WebhookStatus {
                webhook: Webhook {
                    _kind: Default::default(),
                    url: Some("http://test".to_string()),
                    http_method: crate::types::HttpMethod::Get,
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

        let project1 = ProjectId::generate();
        let project2 = ProjectId::generate();
        let t1 = TriggerId::generate(&project1);
        let t2 = TriggerId::generate(&project2);

        let mut i1 = build_invocation(t1.clone(), project1.clone());
        let i2 = build_invocation(t2.clone(), project2.clone());
        let i3 = build_invocation(t1.clone(), project1.clone());

        // Test stores
        store.store_invocation(&i1).await?;
        store.store_invocation(&i2).await?;
        store.store_invocation(&i3).await?;

        // Test getters
        assert_eq!(
            store.get_invocation(&project1, &i1.id).await?,
            Some(i1.clone())
        );
        assert_eq!(
            store.get_invocation(&project2, &i2.id).await?,
            Some(i2.clone())
        );
        assert_eq!(
            store.get_invocation(&project1, &i3.id).await?,
            Some(i3.clone())
        );

        // Test fetching non existent invocation
        assert_eq!(
            store
                .get_invocation(
                    &project1,
                    &InvocationId::from("non_existent".to_string())
                )
                .await?,
            None
        );

        // Test fetching an invocation with wrong project
        assert_eq!(store.get_invocation(&project2, &i1.id).await?, None);

        // Test get invocations by trigger
        let mut results = store
            .get_invocations_by_trigger(&project1, &t1, None, None, 100)
            .await?;
        let mut expected = vec![i1.clone(), i3.clone()];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results, expected);

        // Test get invocation by trigger with wrong project
        assert_eq!(
            store
                .get_invocations_by_trigger(&project2, &t1, None, None, 100)
                .await?,
            vec![]
        );

        // Test get invocations by owner
        let results = store
            .get_invocations_by_project(&project2, None, None, 100)
            .await?;
        let expected = vec![i2.clone()];
        assert_eq!(results, expected);

        i1.status = vec![InvocationStatus::WebhookStatus(WebhookStatus {
            webhook: Webhook {
                _kind: Default::default(),
                url: Some("http://test".to_string()),
                http_method: crate::types::HttpMethod::Get,
                timeout_s: Duration::from_secs(5),
                retry: None,
            },
            delivery_status: WebhookDeliveryStatus::Failed,
        })];

        // Update the invocation
        store.update_invocation(&i1).await?;
        assert_eq!(
            store.get_invocation(&project1, &i1.id).await?,
            Some(i1.clone())
        );

        // Update should fail when using wrong project
        let mut mismatch_project_i1 = i1.clone();
        mismatch_project_i1.project = ProjectId::generate();
        mismatch_project_i1.status = vec![];
        assert!(matches!(
            store.update_invocation(&mismatch_project_i1).await,
            Err(DatabaseError::Query(sqlx::Error::RowNotFound))
        ));
        assert_ne!(
            store.get_invocation(&project1, &i1.id).await?,
            Some(mismatch_project_i1)
        );

        Ok(())
    }
}
