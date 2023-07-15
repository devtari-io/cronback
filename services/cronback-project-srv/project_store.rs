use async_trait::async_trait;
use chrono::Utc;
use lib::database::{Database, DatabaseError};
use lib::prelude::ValidShardedId;
use lib::types::ProjectId;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use crate::db_model::{projects, Project, ProjectStatus, Projects};

pub type ProjectStoreError = DatabaseError;

#[async_trait]
pub trait ProjectStore {
    async fn store_project(
        &self,
        project: Project,
    ) -> Result<(), ProjectStoreError>;

    async fn set_status(
        &self,
        id: &ValidShardedId<ProjectId>,
        status: ProjectStatus,
    ) -> Result<(), ProjectStoreError>;

    async fn get_status(
        &self,
        id: &ValidShardedId<ProjectId>,
    ) -> Result<Option<ProjectStatus>, ProjectStoreError>;

    async fn exists(
        &self,
        id: &ValidShardedId<ProjectId>,
    ) -> Result<bool, ProjectStoreError>;
}

pub struct SqlProjectStore {
    db: Database,
}

impl SqlProjectStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectStore for SqlProjectStore {
    async fn store_project(
        &self,
        project: Project,
    ) -> Result<(), ProjectStoreError> {
        let active_model: projects::ActiveModel = project.into();
        active_model.insert(&self.db.orm).await?;
        Ok(())
    }

    async fn set_status(
        &self,
        id: &ValidShardedId<ProjectId>,
        status: ProjectStatus,
    ) -> Result<(), ProjectStoreError> {
        let active_model = projects::ActiveModel {
            id: Set(id.clone()),
            status: Set(status),
            last_status_changed_at: Set(Utc::now()),
            ..Default::default()
        };

        active_model.update(&self.db.orm).await?;
        Ok(())
    }

    async fn get_status(
        &self,
        id: &ValidShardedId<ProjectId>,
    ) -> Result<Option<ProjectStatus>, ProjectStoreError> {
        Ok(Projects::find_by_id(id.clone())
            .one(&self.db.orm)
            .await?
            .map(|p| p.status))
    }

    async fn exists(
        &self,
        id: &ValidShardedId<ProjectId>,
    ) -> Result<bool, ProjectStoreError> {
        Ok(self.get_status(id).await?.is_some())
    }
}

#[cfg(test)]
mod tests {

    use chrono::{Timelike, Utc};

    use super::{ProjectStore, SqlProjectStore};
    use crate::database::errors::DatabaseError;
    use crate::database::models::projects::ProjectStatus;
    use crate::database::Database;
    use crate::types::{Project, ProjectId};

    fn build_project(status: ProjectStatus) -> Project {
        // Serialization drops nanoseconds, so to let's zero it here for easier
        // equality comparisons
        let now = Utc::now().with_nanosecond(0).unwrap();

        Project {
            id: ProjectId::generate(),
            created_at: now,
            last_status_changed_at: now,
            status,
        }
    }

    #[tokio::test]
    async fn test_sql_project_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlProjectStore::new(db);

        let project1 = build_project(ProjectStatus::Enabled);
        let project2 = build_project(ProjectStatus::QuotaExceeded);

        // Test stores
        store.store_project(project1.clone()).await?;
        store.store_project(project2.clone()).await?;

        // Test exists
        assert!(store.exists(&project1.id).await?);
        assert!(store.exists(&project2.id).await?);
        assert!(!store.exists(&ProjectId::generate()).await?);

        // Test status getters
        assert_eq!(
            store.get_status(&project1.id).await?,
            Some(ProjectStatus::Enabled)
        );
        assert_eq!(
            store.get_status(&project2.id).await?,
            Some(ProjectStatus::QuotaExceeded)
        );
        assert_eq!(store.get_status(&ProjectId::generate()).await?, None);

        // Test status setters
        store
            .set_status(&project2.id, ProjectStatus::Disabled)
            .await?;
        assert_eq!(
            store.get_status(&project2.id).await?,
            Some(ProjectStatus::Disabled)
        );

        // Test status setter for non-existent project
        assert!(matches!(
            store
                .set_status(
                    &ProjectId::generate(),
                    ProjectStatus::PendingDeletion
                )
                .await,
            Err(DatabaseError::DB(sea_orm::DbErr::RecordNotUpdated))
        ));

        Ok(())
    }
}
