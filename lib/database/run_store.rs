use async_trait::async_trait;
use proto::common::PaginationIn;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use super::errors::DatabaseError;
use super::models::runs;
use super::pagination::{PaginatedResponse, PaginatedSelect};
use crate::database::models::prelude::Runs;
use crate::database::Database;
use crate::model::ModelId;
use crate::types::{ProjectId, Run, RunId, TriggerId};

pub type RunStoreError = DatabaseError;

#[async_trait]
pub trait RunStore {
    async fn store_run(&self, run: Run) -> Result<(), RunStoreError>;

    async fn update_run(&self, run: Run) -> Result<(), RunStoreError>;

    async fn get_run(
        &self,
        project: &ProjectId,
        id: &RunId,
    ) -> Result<Option<Run>, RunStoreError>;

    async fn get_runs_by_trigger(
        &self,
        project: &ProjectId,
        trigger_id: &TriggerId,
        pagination: PaginationIn,
    ) -> Result<PaginatedResponse<Run>, RunStoreError>;

    async fn get_runs_by_project(
        &self,
        project: &ProjectId,
        pagination: PaginationIn,
    ) -> Result<PaginatedResponse<Run>, RunStoreError>;
}

pub struct SqlRunStore {
    db: Database,
}

impl SqlRunStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RunStore for SqlRunStore {
    async fn store_run(&self, run: Run) -> Result<(), RunStoreError> {
        let active_model: runs::ActiveModel = run.into();
        active_model.insert(&self.db.orm).await?;
        Ok(())
    }

    async fn update_run(&self, run: Run) -> Result<(), RunStoreError> {
        let project = run.project_id.clone();
        let active_model: runs::ActiveModel = run.into();
        // Mark all the fields as dirty
        let active_model = active_model.reset_all();
        Runs::update(active_model)
            .filter(runs::Column::ProjectId.eq(project))
            .exec(&self.db.orm)
            .await?;
        Ok(())
    }

    async fn get_run(
        &self,
        project: &ProjectId,
        id: &RunId,
    ) -> Result<Option<Run>, RunStoreError> {
        let res = Runs::find_by_id((id.to_string(), project.to_string()))
            .one(&self.db.orm)
            .await?;
        Ok(res)
    }

    async fn get_runs_by_trigger(
        &self,
        project: &ProjectId,
        trigger_id: &TriggerId,
        pagination: PaginationIn,
    ) -> Result<PaginatedResponse<Run>, RunStoreError> {
        let query = Runs::find()
            .filter(runs::Column::TriggerId.eq(trigger_id.value()))
            .filter(runs::Column::ProjectId.eq(project.value()))
            .with_pagination(&pagination);

        let res = query.all(&self.db.orm).await?;
        Ok(PaginatedResponse::paginate(res, &pagination))
    }

    async fn get_runs_by_project(
        &self,
        project: &ProjectId,
        pagination: PaginationIn,
    ) -> Result<PaginatedResponse<Run>, RunStoreError> {
        let query = Runs::find()
            .filter(runs::Column::ProjectId.eq(project.value()))
            .with_pagination(&pagination);

        let res = query.all(&self.db.orm).await?;

        Ok(PaginatedResponse::paginate(res, &pagination))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Timelike, Utc};
    use proto::common::PaginationIn;
    use sea_orm::DbErr;

    use super::{RunStore, SqlRunStore};
    use crate::database::errors::DatabaseError;
    use crate::database::Database;
    use crate::model::ValidShardedId;
    use crate::types::{
        Action,
        ProjectId,
        Run,
        RunId,
        RunStatus,
        TriggerId,
        Webhook,
    };

    fn build_run(
        trigger_id: ValidShardedId<TriggerId>,
        project: ValidShardedId<ProjectId>,
    ) -> Run {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_nanosecond(0).unwrap();

        Run {
            id: RunId::generate(&project).into(),
            trigger_id: trigger_id.into(),
            project_id: project,
            created_at: now,
            action: Action::Webhook(Webhook {
                url: "http://test".to_string(),
                http_method: crate::types::HttpMethod::Get,
                timeout_s: Duration::from_secs(5),
                retry: None,
            }),
            payload: None,
            status: RunStatus::Attempting,
        }
    }

    #[tokio::test]
    async fn test_sql_run_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlRunStore::new(db);

        let project1 = ProjectId::generate();
        let project2 = ProjectId::generate();
        let t1 = TriggerId::generate(&project1);
        let t2 = TriggerId::generate(&project2);

        let mut i1 = build_run(t1.clone(), project1.clone());
        let i2 = build_run(t2.clone(), project2.clone());
        let i3 = build_run(t1.clone(), project1.clone());

        // Test stores
        store.store_run(i1.clone()).await?;
        store.store_run(i2.clone()).await?;
        store.store_run(i3.clone()).await?;

        // Test getters
        assert_eq!(store.get_run(&project1, &i1.id).await?, Some(i1.clone()));
        assert_eq!(store.get_run(&project2, &i2.id).await?, Some(i2.clone()));
        assert_eq!(store.get_run(&project1, &i3.id).await?, Some(i3.clone()));

        // Test fetching non existent run
        assert_eq!(
            store
                .get_run(&project1, &RunId::from("non_existent".to_string()))
                .await?,
            None
        );

        // Test fetching a run with wrong project
        assert_eq!(store.get_run(&project2, &i1.id).await?, None);

        // Test get runs by trigger
        let mut results = store
            .get_runs_by_trigger(&project1, &t1, PaginationIn::default())
            .await?;
        let mut expected = vec![i1.clone(), i3.clone()];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.data.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results.data, expected);

        // Test get run by trigger with wrong project
        assert_eq!(
            store
                .get_runs_by_trigger(&project2, &t1, PaginationIn::default())
                .await?
                .data,
            vec![]
        );

        // Test get runs by owner
        let results = store
            .get_runs_by_project(&project2, PaginationIn::default())
            .await?;
        let expected = vec![i2.clone()];
        assert_eq!(results.data, expected);

        i1.status = RunStatus::Failed;

        // Update the run
        store.update_run(i1.clone()).await?;
        assert_eq!(store.get_run(&project1, &i1.id).await?, Some(i1.clone()));

        // Update should fail when using wrong project
        let mut mismatch_project_i1 = i1.clone();
        mismatch_project_i1.project_id = ProjectId::generate();
        mismatch_project_i1.status = RunStatus::Succeeded;
        assert!(matches!(
            store.update_run(mismatch_project_i1.clone()).await,
            Err(DatabaseError::DB(DbErr::RecordNotUpdated))
        ));
        assert_ne!(
            store.get_run(&project1, &i1.id).await?,
            Some(mismatch_project_i1)
        );

        Ok(())
    }
}
