use async_trait::async_trait;
use sea_query::{Alias, ColumnDef, Expr, Iden, Query, Table};
use sea_query_binder::SqlxBinder;
use sqlx::Row;
use tracing::debug;

use super::errors::DatabaseError;
use super::helpers::{
    get_by_id_query,
    insert_query,
    paginated_query,
    JsonField,
    KVIden,
};
use crate::database::Database;
use crate::types::{ProjectId, ShardedId, Status, Trigger, TriggerId};

pub type TriggerStoreError = DatabaseError;

#[derive(Iden)]
enum TriggersIden {
    Triggers,
}

#[async_trait]
pub trait TriggerStore {
    async fn install_trigger(
        &self,
        trigger: &Trigger,
    ) -> Result<(), TriggerStoreError>;

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError>;

    async fn get_trigger(
        &self,
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

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        let (sql, values) = Query::select()
            .columns([KVIden::Id, KVIden::Value])
            .from(TriggersIden::Triggers)
            .and_where(
                Expr::expr(
                    Expr::col(KVIden::Value)
                        .cast_json_field("status", self.db.pool.any_kind()),
                )
                .is_in(["active", "paused"]),
            )
            .build_any_sqlx(self.db.builder().as_ref());

        let results = sqlx::query_with(&sql, values)
            .fetch_all(&self.db.pool)
            .await?
            .into_iter()
            .map(|r| {
                let id = r.get::<String, _>(KVIden::Id.to_string().as_str());
                debug!(trigger_id = %id, "Loading trigger from database");
                let j = r.get::<String, _>(KVIden::Value.to_string().as_str());
                serde_json::from_str::<Trigger>(&j)
            })
            .collect::<Result<Vec<_>, _>>();
        Ok(results?)
    }

    async fn get_triggers_by_project(
        &self,
        project: &ProjectId,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
        limit: usize,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        paginated_query(
            &self.db,
            TriggersIden::Triggers,
            "project",
            project.value(),
            &before,
            &after,
            limit,
        )
        .await
    }

    async fn get_trigger(
        &self,
        id: &TriggerId,
    ) -> Result<Option<Trigger>, TriggerStoreError> {
        get_by_id_query(&self.db, TriggersIden::Triggers, id).await
    }

    async fn get_status(
        &self,
        project: &ProjectId,
        id: &TriggerId,
    ) -> Result<Option<Status>, TriggerStoreError> {
        let (sql, values) = Query::select()
            .column(KVIden::Id)
            .expr_as(
                Expr::col(KVIden::Value)
                    .get_json_field("status", self.db.pool.any_kind()),
                Alias::new("status"),
            )
            .from(TriggersIden::Triggers)
            .and_where(Expr::col(KVIden::Id).eq(id.to_string()))
            .and_where(
                Expr::col(KVIden::Value)
                    .cast_json_field("project", self.db.pool.any_kind())
                    .eq(project.to_string()),
            )
            .build_any_sqlx(self.db.builder().as_ref());

        let result = sqlx::query_with(&sql, values)
            .fetch_one(&self.db.pool)
            .await;

        match result {
            | Ok(r) => {
                let j = r.get::<String, _>("status");
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
    use crate::database::Database;
    use crate::types::{Emit, ProjectId, Status, Trigger, TriggerId, Webhook};

    fn build_trigger(
        name: &str,
        project: ProjectId,
        status: Status,
    ) -> Trigger {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_nanosecond(0).unwrap();

        Trigger {
            id: TriggerId::new(&project),
            project,
            name: name.to_string(),
            description: Some(format!("Desc: {}", name)),
            created_at: now,
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

        let owner1 = ProjectId::new();
        let owner2 = ProjectId::new();

        let t1 = build_trigger("t1", owner1.clone(), Status::Active);
        let t2 = build_trigger("t2", owner1.clone(), Status::Paused);
        let t3 = build_trigger("t3", owner2.clone(), Status::Active);
        let t4 = build_trigger("t4", owner2.clone(), Status::Expired);

        // Test installs
        store.install_trigger(&t1).await?;
        store.install_trigger(&t2).await?;
        store.install_trigger(&t3).await?;
        store.install_trigger(&t4).await?;

        // Test getters
        assert_eq!(store.get_trigger(&t1.id).await?, Some(t1.clone()));
        assert_eq!(store.get_trigger(&t2.id).await?, Some(t2.clone()));
        assert_eq!(store.get_trigger(&t3.id).await?, Some(t3.clone()));
        assert_eq!(store.get_trigger(&t4.id).await?, Some(t4.clone()));

        // Test fetching non existent trigger
        assert_eq!(
            store
                .get_trigger(&TriggerId::from("non_existent".to_string()))
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
            .get_triggers_by_project(&owner1, None, None, 100)
            .await?;
        let mut expected = vec![t1.clone(), t2.clone()];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results, expected);

        // Test Get Status
        assert_eq!(
            Some(Status::Active),
            store.get_status(&owner1, &t1.id).await?
        );
        assert_eq!(
            Some(Status::Paused),
            store.get_status(&owner1, &t2.id).await?
        );

        Ok(())
    }
}
