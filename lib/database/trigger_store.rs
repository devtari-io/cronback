use async_trait::async_trait;
use sea_query::{ColumnDef, Expr, Iden, Index, Query, Table};
use sea_query_binder::SqlxBinder;
use serde_json::json;
use sqlx::Row;

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
use crate::types::{ProjectId, Status, Trigger, TriggerId};

pub type TriggerStoreError = DatabaseError;

#[derive(Iden)]
enum TriggersIden {
    Triggers,
    Reference,
    Status,
}

#[async_trait]
pub trait TriggerStore {
    async fn install_trigger(
        &self,
        trigger: &Trigger,
    ) -> Result<(), TriggerStoreError>;

    async fn update_trigger(
        &self,
        trigger: &Trigger,
    ) -> Result<(), TriggerStoreError>;

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError>;

    async fn get_trigger(
        &self,
        project: &ProjectId,
        id: &TriggerId,
    ) -> Result<Option<Trigger>, TriggerStoreError>;

    async fn get_status(
        &self,
        project: &ProjectId,
        id: &TriggerId,
    ) -> Result<Option<Status>, TriggerStoreError>;

    async fn get_triggers_by_project(
        &self,
        project: &ProjectId,
        reference: Option<String>,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
        limit: usize,
    ) -> Result<Vec<Trigger>, TriggerStoreError>;
}

pub struct SqlTriggerStore {
    db: Database,
}

impl SqlTriggerStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn prepare(&self) -> Result<(), TriggerStoreError> {
        let sql = Table::create()
            .table(TriggersIden::Triggers)
            .if_not_exists()
            .col(ColumnDef::new(KVIden::Id).text().primary_key())
            .col(ColumnDef::new(KVIden::Value).json_binary())
            .col(
                ColumnDef::new(KVIden::Project)
                    .text()
                    .generate_from_json_field(KVIden::Value, "project"),
            )
            .col(
                ColumnDef::new(TriggersIden::Reference)
                    .text()
                    .generate_from_json_field(KVIden::Value, "reference"),
            )
            .col(
                ColumnDef::new(TriggersIden::Status)
                    .text()
                    .generate_from_json_field(KVIden::Value, "status"),
            )
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;

        // Create the indicies
        let sql = Index::create()
            .if_not_exists()
            .name("IX_triggers_project")
            .table(TriggersIden::Triggers)
            .col(KVIden::Project)
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;

        let sql = Index::create()
            .if_not_exists()
            .name("UQ_triggers_project_reference")
            .table(TriggersIden::Triggers)
            .col(KVIden::Project)
            .col(TriggersIden::Reference)
            .unique()
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;

        let sql = Index::create()
            .if_not_exists()
            .name("IX_triggers_status")
            .table(TriggersIden::Triggers)
            .col(TriggersIden::Status)
            .build_any(self.db.schema_builder().as_ref());

        sqlx::query(&sql).execute(&self.db.pool).await?;

        Ok(())
    }
}

#[async_trait]
impl TriggerStore for SqlTriggerStore {
    async fn install_trigger(
        &self,
        trigger: &Trigger,
    ) -> Result<(), TriggerStoreError> {
        insert_query(&self.db, TriggersIden::Triggers, &trigger.id, trigger)
            .await
    }

    async fn update_trigger(
        &self,
        trigger: &Trigger,
    ) -> Result<(), TriggerStoreError> {
        update_query(&self.db, TriggersIden::Triggers, &trigger.id, trigger)
            .await
    }

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        paginated_query(
            &self.db,
            TriggersIden::Triggers,
            Expr::col(TriggersIden::Status).is_in(["scheduled", "paused"]),
            &Option::<TriggerId>::None,
            &Option::<TriggerId>::None,
            None,
        )
        .await
    }

    async fn get_triggers_by_project(
        &self,
        project: &ProjectId,
        reference: Option<String>,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
        limit: usize,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        let mut condition = Expr::col(KVIden::Project).eq(project.value());
        if let Some(reference) = reference {
            condition =
                condition.and(Expr::col(TriggersIden::Reference).eq(reference));
        }
        paginated_query(
            &self.db,
            TriggersIden::Triggers,
            condition,
            &before,
            &after,
            Some(limit),
        )
        .await
    }

    async fn get_trigger(
        &self,
        project: &ProjectId,
        id: &TriggerId,
    ) -> Result<Option<Trigger>, TriggerStoreError> {
        let t = get_by_id_query(&self.db, TriggersIden::Triggers, id).await?;
        Ok(t.filter(|x: &Trigger| x.project.inner() == project))
    }

    async fn get_status(
        &self,
        project: &ProjectId,
        id: &TriggerId,
    ) -> Result<Option<Status>, TriggerStoreError> {
        let (sql, values) = Query::select()
            .column(KVIden::Id)
            .column(TriggersIden::Status)
            .from(TriggersIden::Triggers)
            .and_where(Expr::col(KVIden::Id).eq(id.to_string()))
            .and_where(Expr::col(KVIden::Project).eq(project.to_string()))
            .build_any_sqlx(self.db.builder().as_ref());

        let result = sqlx::query_with(&sql, values)
            .fetch_one(&self.db.pool)
            .await;

        match result {
            | Ok(r) => {
                let j = json!(r.get::<String, _>("status")).to_string();
                Ok(Some(serde_json::from_str::<Status>(&j)?))
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

    use super::{SqlTriggerStore, TriggerStore};
    use crate::database::trigger_store::TriggerStoreError;
    use crate::database::Database;
    use crate::model::ValidShardedId;
    use crate::types::{Emit, ProjectId, Status, Trigger, TriggerId, Webhook};

    fn build_trigger(
        name: &str,
        project: ValidShardedId<ProjectId>,
        status: Status,
    ) -> Trigger {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_nanosecond(0).unwrap();

        Trigger {
            id: TriggerId::generate(&project).into(),

            project,
            name: name.to_string(),
            description: Some(format!("Desc: {}", name)),
            created_at: now,
            updated_at: None,
            payload: None,
            schedule: None,
            emit: vec![Emit::Webhook(Webhook {
                _kind: Default::default(),
                url: Some("http://test".to_string()),
                http_method: crate::types::HttpMethod::GET,
                timeout_s: Duration::from_secs(5),
                retry: None,
            })],
            status,
            reference: None,
            last_invoked_at: None,
        }
    }

    #[tokio::test]
    async fn test_sql_trigger_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlTriggerStore::new(db);
        store.prepare().await?;

        let owner1 = ProjectId::generate();

        let owner2 = ProjectId::generate();

        let t1 = build_trigger("t1", owner1.clone(), Status::Scheduled);
        let t2 = build_trigger("t2", owner1.clone(), Status::Paused);
        let t3 = build_trigger("t3", owner2.clone(), Status::Scheduled);
        let t4 = build_trigger("t4", owner2.clone(), Status::Expired);

        // Test installs
        store.install_trigger(&t1).await?;
        store.install_trigger(&t2).await?;
        store.install_trigger(&t3).await?;
        store.install_trigger(&t4).await?;

        // Test getters
        assert_eq!(store.get_trigger(&owner1, &t1.id).await?, Some(t1.clone()));
        assert_eq!(store.get_trigger(&owner1, &t2.id).await?, Some(t2.clone()));
        assert_eq!(store.get_trigger(&owner2, &t3.id).await?, Some(t3.clone()));
        assert_eq!(store.get_trigger(&owner2, &t4.id).await?, Some(t4.clone()));
        // Wrong project.
        assert_eq!(store.get_trigger(&owner1, &t4.id).await?, None);

        // Test fetching non existent trigger
        assert_eq!(
            store
                .get_trigger(
                    &owner1,
                    &TriggerId::from("non_existent".to_string())
                )
                .await?,
            None
        );

        // Test get all active
        let mut results = store.get_all_active_triggers().await?;
        let mut expected = vec![t1.clone(), t2.clone(), t3.clone()];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results, expected);

        // Test get by owner
        let mut results = store
            .get_triggers_by_project(&owner1, None, None, None, 100)
            .await?;
        let mut expected = vec![t1.clone(), t2.clone()];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results, expected);

        // Test Get Status
        assert_eq!(
            Some(Status::Scheduled),
            store.get_status(&owner1, &t1.id).await?
        );
        assert_eq!(
            Some(Status::Paused),
            store.get_status(&owner1, &t2.id).await?
        );

        // Test reference uniqueness
        let mut t5 = build_trigger("t5", owner1.clone(), Status::Scheduled);
        t5.reference = Some("Ref".to_string());
        let t6 = t5.clone();
        store.install_trigger(&t5).await?;
        assert!(matches!(
            store.install_trigger(&t6).await,
            Err(TriggerStoreError::DuplicateRecord)
        ));

        // Test get by reference
        let results = store
            .get_triggers_by_project(
                &owner1,
                Some("Ref".to_string()),
                None,
                None,
                100,
            )
            .await?;

        assert_eq!(results, vec![t5.clone()]);

        Ok(())
    }
}
