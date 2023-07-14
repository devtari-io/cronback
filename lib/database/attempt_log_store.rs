use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait,
    ColumnTrait,
    EntityTrait,
    QueryFilter,
    QueryOrder,
    QuerySelect,
};

use super::errors::DatabaseError;
use super::models::attempts;
use crate::database::models::prelude::Attempts;
use crate::database::Database;
use crate::model::ModelId;
use crate::types::{ActionAttemptLog, AttemptLogId, ProjectId, RunId};

pub type AttemptLogStoreError = DatabaseError;

#[async_trait]
pub trait AttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: ActionAttemptLog,
    ) -> Result<(), AttemptLogStoreError>;

    async fn get_attempts_for_run(
        &self,
        project: &ProjectId,
        id: &RunId,
        before: Option<AttemptLogId>,
        after: Option<AttemptLogId>,
        limit: usize,
    ) -> Result<Vec<ActionAttemptLog>, AttemptLogStoreError>;

    async fn get_attempt(
        &self,
        project: &ProjectId,
        id: &AttemptLogId,
    ) -> Result<Option<ActionAttemptLog>, AttemptLogStoreError>;
}

pub struct SqlAttemptLogStore {
    db: Database,
}

impl SqlAttemptLogStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AttemptLogStore for SqlAttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: ActionAttemptLog,
    ) -> Result<(), AttemptLogStoreError> {
        let active_model: attempts::ActiveModel = attempt.into();
        active_model.insert(&self.db.orm).await?;
        Ok(())
    }

    async fn get_attempts_for_run(
        &self,
        project: &ProjectId,
        id: &RunId,
        before: Option<AttemptLogId>,
        after: Option<AttemptLogId>,
        limit: usize,
    ) -> Result<Vec<ActionAttemptLog>, AttemptLogStoreError> {
        let mut query = Attempts::find()
            .filter(attempts::Column::RunId.eq(id.value()))
            .filter(attempts::Column::ProjectId.eq(project.value()))
            .order_by_desc(attempts::Column::Id)
            .limit(Some(limit as u64));
        if let Some(before) = before {
            query = query.filter(attempts::Column::Id.gt(before.value()));
        }

        if let Some(after) = after {
            query = query.filter(attempts::Column::Id.lt(after.value()));
        }

        let res = query.all(&self.db.orm).await?;

        Ok(res)
    }

    async fn get_attempt(
        &self,
        project_id: &ProjectId,
        id: &AttemptLogId,
    ) -> Result<Option<ActionAttemptLog>, AttemptLogStoreError> {
        let res =
            Attempts::find_by_id((id.to_string(), project_id.to_string()))
                .one(&self.db.orm)
                .await?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Timelike, Utc};

    use super::{AttemptLogStore, SqlAttemptLogStore};
    use crate::database::Database;
    use crate::model::ValidShardedId;
    use crate::types::{
        ActionAttemptLog,
        AttemptDetails,
        AttemptLogId,
        AttemptStatus,
        ProjectId,
        RunId,
        TriggerId,
        WebhookAttemptDetails,
    };

    fn build_attempt(
        project: &ValidShardedId<ProjectId>,
        run_id: &RunId,
    ) -> ActionAttemptLog {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_nanosecond(0).unwrap();

        ActionAttemptLog {
            id: AttemptLogId::generate(project).into(),
            run_id: run_id.clone(),
            trigger_id: TriggerId::generate(project).into(),
            project_id: project.clone(),
            status: AttemptStatus::Succeeded,
            details: AttemptDetails::WebhookAttemptDetails(
                WebhookAttemptDetails {
                    response_code: Some(404),
                    response_latency_s: Duration::from_secs(10),
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

        let project = ProjectId::generate();
        let project2 = ProjectId::generate();
        let inv1 = RunId::generate(&project);
        let inv2 = RunId::generate(&project);

        let a1 = build_attempt(&project, &inv1);
        let a2 = build_attempt(&project, &inv2);
        let a3 = build_attempt(&project, &inv1);

        // Test log attempts
        store.log_attempt(a1.clone()).await?;
        store.log_attempt(a2.clone()).await?;
        store.log_attempt(a3.clone()).await?;

        // Test getters
        assert_eq!(
            store.get_attempt(&project, &a1.id).await?,
            Some(a1.clone())
        );
        assert_eq!(
            store.get_attempt(&project, &a2.id).await?,
            Some(a2.clone())
        );
        assert_eq!(
            store.get_attempt(&project, &a3.id).await?,
            Some(a3.clone())
        );

        // Test fetching non existent attempt
        assert_eq!(
            store
                .get_attempt(
                    &project,
                    &AttemptLogId::from("non_existent".to_string())
                )
                .await?,
            None
        );

        // Test fetching an attempt with wrong project
        assert_eq!(store.get_attempt(&project2, &a1.id).await?, None);

        // Test get all attempts for a certain run
        let mut results = store
            .get_attempts_for_run(&project, &inv1, None, None, 100)
            .await?;
        let mut expected = vec![a1, a3];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(results, expected);

        // Test get all attempts for a certain run with a wrong project
        assert_eq!(
            store
                .get_attempts_for_run(&project2, &inv1, None, None, 100)
                .await?,
            vec![]
        );

        Ok(())
    }
}
