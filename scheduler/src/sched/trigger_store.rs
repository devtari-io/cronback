use async_trait::async_trait;
use shared::database::SqliteDatabase;
use shared::types::{Trigger, TriggerId};
use sqlx::Row;
use thiserror::Error;
use tracing::debug;

#[derive(Error, Debug)]
pub enum TriggerStoreError {
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    ParseError(#[from] serde_json::Error),
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
        sqlx::query("INSERT OR REPLACE INTO triggers (id,value) VALUES (?,?)")
            .bind(&trigger.id.to_string())
            .bind(serde_json::to_string(trigger)?)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        let results = sqlx::query(
            "SELECT id, value FROM triggers where JSON_EXTRACT(value, \
             '$.status') = 'active'",
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

    async fn get_trigger(
        &self,
        id: &TriggerId,
    ) -> Result<Option<Trigger>, TriggerStoreError> {
        let result = sqlx::query("SELECT value FROM triggers WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.db.pool)
            .await;

        match result {
            | Ok(r) => {
                let j = r.get::<String, _>("value");
                Ok(Some(serde_json::from_str::<Trigger>(&j)?))
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
    use shared::database::SqliteDatabase;
    use shared::types::{
        Emit,
        OwnerId,
        Payload,
        Status,
        Trigger,
        TriggerId,
        Webhook,
    };

    use super::{SqlTriggerStore, TriggerStore};

    fn build_trigger(name: &str, status: Status) -> Trigger {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_nanosecond(0).unwrap();

        let owner = OwnerId::new();
        Trigger {
            id: TriggerId::new(&owner),
            owner_id: owner,
            name: Some(name.to_string()),
            description: Some(format!("Desc: {}", name)),
            created_at: now,
            payload: Payload {
                ..Default::default()
            },
            schedule: None,
            emit: vec![Emit::Webhook(Webhook {
                url: Some("http://test".to_string()),
                http_method: shared::types::HttpMethod::GET,
                timeout_s: Duration::from_secs(5),
                retry: None,
            })],
            status,
            reference_id: None,
            hidden_last_invoked_at: None,
        }
    }

    #[tokio::test]
    async fn test_sql_trigger_store() -> anyhow::Result<()> {
        let db = SqliteDatabase::in_memory().await?;
        let store = SqlTriggerStore::create(db).await?;

        let t1 = build_trigger("t1", Status::Active);
        let t2 = build_trigger("t2", Status::Paused);
        let t3 = build_trigger("t3", Status::Active);

        // Test installs
        store.install_trigger(&t1).await?;
        store.install_trigger(&t2).await?;
        store.install_trigger(&t3).await?;

        // Test getters
        assert_eq!(store.get_trigger(&t1.id).await?, Some(t1.clone()));
        assert_eq!(store.get_trigger(&t2.id).await?, Some(t2.clone()));
        assert_eq!(store.get_trigger(&t3.id).await?, Some(t3.clone()));

        // Test fetching non existent trigger
        assert_eq!(
            store
                .get_trigger(&TriggerId::from("non_existent".to_string()))
                .await?,
            None
        );

        // Test get all active
        let mut results = store.get_all_active_triggers().await?;
        let mut expected = vec![t1, t3];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(results, expected);

        Ok(())
    }
}
