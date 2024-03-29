use chrono::Utc;
use lib::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use super::db_model::notifications::NotificationSettings;
use super::db_model::{projects, Project, ProjectStatus, Projects};

pub type MetadataStoreError = DatabaseError;

#[derive(Clone)]
pub struct MetadataStore {
    db: Database,
}

impl MetadataStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn store_project(
        &self,
        project: Project,
    ) -> Result<(), MetadataStoreError> {
        let active_model: projects::ActiveModel = project.into();
        active_model.insert(&self.db.orm).await?;
        Ok(())
    }

    pub async fn set_status(
        &self,
        id: &ValidShardedId<ProjectId>,
        status: ProjectStatus,
    ) -> Result<(), MetadataStoreError> {
        let active_model = projects::ActiveModel {
            id: Set(id.clone()),
            status: Set(status),
            changed_at: Set(Utc::now()),
            ..Default::default()
        };

        active_model.update(&self.db.orm).await?;
        Ok(())
    }

    pub async fn get_status(
        &self,
        id: &ValidShardedId<ProjectId>,
    ) -> Result<Option<ProjectStatus>, MetadataStoreError> {
        Ok(Projects::find_by_id(id.clone())
            .one(&self.db.orm)
            .await?
            .map(|p| p.status))
    }

    pub async fn set_notification_settings(
        &self,
        id: &ValidShardedId<ProjectId>,
        settings: NotificationSettings,
    ) -> Result<(), MetadataStoreError> {
        let active_model = projects::ActiveModel {
            id: Set(id.clone()),
            notification_settings: Set(settings),
            changed_at: Set(Utc::now()),
            ..Default::default()
        };

        active_model.update(&self.db.orm).await?;
        Ok(())
    }

    pub async fn get_notification_settings(
        &self,
        id: &ValidShardedId<ProjectId>,
    ) -> Result<Option<NotificationSettings>, MetadataStoreError> {
        Ok(Projects::find_by_id(id.clone())
            .one(&self.db.orm)
            .await?
            .map(|p| p.notification_settings))
    }

    pub async fn exists(
        &self,
        id: &ValidShardedId<ProjectId>,
    ) -> Result<bool, MetadataStoreError> {
        Ok(self.get_status(id).await?.is_some())
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::*;
    use crate::metadata::db_model::notifications::{
        EmailNotification,
        NotificationChannel,
        NotificationEvent,
        NotificationSubscription,
        OnRunFailure,
    };
    use crate::metadata::MetadataService;

    fn build_project(status: ProjectStatus) -> Project {
        let now = Utc::now();

        Project {
            id: ProjectId::generate(),
            created_at: now,
            changed_at: now,
            status,
            notification_settings: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_sql_project_store() -> anyhow::Result<()> {
        let db = MetadataService::in_memory_database().await?;
        let store = MetadataStore::new(db);

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

        // Test notification setters / getters
        {
            let email = EmailNotification {
                address: "test@gmail.com".to_string(),
                verified: true,
            };
            let mut channels = HashMap::new();
            channels
                .insert("email".to_string(), NotificationChannel::Email(email));
            let setting = NotificationSettings {
                channels,
                default_subscriptions: vec![NotificationSubscription {
                    channel_names: vec!["email".to_string()],
                    event: NotificationEvent::OnRunFailure(OnRunFailure {}),
                }],
            };
            store
                .set_notification_settings(&project2.id, setting.clone())
                .await?;

            let found = store.get_notification_settings(&project2.id).await?;
            assert_eq!(found, Some(setting));
        }

        Ok(())
    }
}
