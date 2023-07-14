use async_trait::async_trait;
use sqlx::Row;
use tracing::debug;

use super::errors::DatabaseError;
use super::helpers::{get_by_id_query, insert_query, paginated_query};
use crate::database::SqliteDatabase;
use crate::types::{ProjectId, ShardedId, Status, Trigger, TriggerId};

pub type TriggerStoreError = DatabaseError;

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

    async fn get_triggers_by_owner(
        &self,
        project: &ProjectId,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
        limit: usize,
    ) -> Result<Vec<Trigger>, TriggerStoreError>;
}

pub struct SqlTriggerStore {
    db: SqliteDatabase,
}

impl SqlTriggerStore {
    pub async fn create(db: SqliteDatabase) -> Result<Self, TriggerStoreError> {
        let s = Self { db };
        s.prepare().await?;
        Ok(s)
    }

    async fn prepare(&self) -> Result<(), TriggerStoreError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS triggers (
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
impl TriggerStore for SqlTriggerStore {
    async fn install_trigger(
        &self,
        trigger: &Trigger,
    ) -> Result<(), TriggerStoreError> {
        insert_query(&self.db.pool, "triggers", &trigger.id, trigger).await
    }

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        let results = sqlx::query(
            "SELECT id, value FROM triggers where JSON_EXTRACT(value, \
             '$.status') IN ('active', 'paused')",
        )
        .fetch_all(&self.db.pool)
        .await?
        .into_iter()
        .map(|r| {
            let id = r.get::<String, _>("id");
            debug!(trigger_id = %id, "Loading trigger from database");
            let j = r.get::<String, _>("value");
            serde_json::from_str::<Trigger>(&j)
        })
        .collect::<Result<Vec<_>, _>>();
        Ok(results?)
    }

    async fn get_triggers_by_owner(
        &self,
        project: &ProjectId,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
        limit: usize,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        paginated_query(
            &self.db.pool,
            "triggers",
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
        get_by_id_query(&self.db.pool, "triggers", id).await
    }

    async fn get_status(
        &self,
        project: &ProjectId,
        id: &TriggerId,
    ) -> Result<Option<Status>, TriggerStoreError> {
        let result = sqlx::query(
            "SELECT id, value->'$.status' AS status FROM triggers WHERE id = \
             ? AND JSON_EXTRACT(value, '$.project') = ?",
        )
        .bind(id.to_string())
        .bind(project.to_string())
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
    use crate::database::SqliteDatabase;
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
        let db = SqliteDatabase::in_memory().await?;
        let store = SqlTriggerStore::create(db).await?;

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
            .get_triggers_by_owner(&owner1, None, None, 100)
            .await?;
        let mut expected = vec![t1, t2];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results, expected);

        Ok(())
    }
}
