use lib::prelude::*;
#[cfg(test)]
use proto::common::PaginationIn;
use sea_orm::ActiveModelTrait;
#[cfg(test)]
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[cfg(test)]
use super::db_model::Attempts;
use super::db_model::{attempts, Attempt};

pub type AttemptStoreError = DatabaseError;

#[derive(Clone)]
pub struct AttemptStore {
    db: Database,
}

impl AttemptStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn log_attempt(
        &self,
        attempt: Attempt,
    ) -> Result<(), AttemptStoreError> {
        let active_model: attempts::ActiveModel = attempt.into();
        active_model.insert(&self.db.orm).await?;
        Ok(())
    }

    // Leaving this under cfg(test) until someone actually need it.
    #[cfg(test)]
    pub async fn get_attempts_for_run(
        &self,
        project: &ValidShardedId<ProjectId>,
        id: &RunId,
        pagination: PaginationIn,
    ) -> Result<PaginatedResponse<Attempt>, AttemptStoreError> {
        let query = Attempts::find()
            .filter(attempts::Column::RunId.eq(id.value()))
            .filter(attempts::Column::ProjectId.eq(project.value()))
            .with_pagination(&pagination);

        let res = query.all(&self.db.orm).await?;

        Ok(PaginatedResponse::paginate(res, &pagination))
    }

    // Leaving this under cfg(test) until someone actually need it.
    #[cfg(test)]
    pub async fn get_attempt(
        &self,
        project_id: &ValidShardedId<ProjectId>,
        id: &AttemptId,
    ) -> Result<Option<Attempt>, AttemptStoreError> {
        let res = Attempts::find_by_id((id.clone(), project_id.clone()))
            .one(&self.db.orm)
            .await?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Timelike, Utc};

    use super::*;
    use crate::dispatcher::db_model::attempts::{
        AttemptDetails,
        AttemptStatus,
        WebhookAttemptDetails,
    };
    use crate::dispatcher::DispatcherService;

    fn build_attempt(
        project: &ValidShardedId<ProjectId>,
        run_id: &RunId,
    ) -> Attempt {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_nanosecond(0).unwrap();

        Attempt {
            id: AttemptId::generate(project).into(),
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
            attempt_num: 5,
            created_at: now,
        }
    }

    #[tokio::test]
    async fn test_trigger_store() -> anyhow::Result<()> {
        let db = DispatcherService::in_memory_database().await?;
        let store = AttemptStore::new(db);

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
                    &AttemptId::from("non_existent".to_string())
                )
                .await?,
            None
        );

        // Test fetching an attempt with wrong project
        assert_eq!(store.get_attempt(&project2, &a1.id).await?, None);

        // Test get all attempts for a certain run
        let mut results = store
            .get_attempts_for_run(&project, &inv1, PaginationIn::default())
            .await?;
        let mut expected = vec![a1, a3];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.data.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(results.data, expected);

        // Test get all attempts for a certain run with a wrong project
        assert_eq!(
            store
                .get_attempts_for_run(&project2, &inv1, PaginationIn::default())
                .await?
                .data,
            vec![]
        );

        Ok(())
    }
}
