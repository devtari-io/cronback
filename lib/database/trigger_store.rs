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
use super::models::triggers::{self, Status};
use crate::database::models::prelude::Triggers;
use crate::database::Database;
use crate::model::ModelId;
use crate::types::{ProjectId, Trigger, TriggerId};

pub type TriggerStoreError = DatabaseError;

#[async_trait]
pub trait TriggerStore {
    async fn install_trigger(
        &self,
        trigger: Trigger,
    ) -> Result<(), TriggerStoreError>;

    async fn update_trigger(
        &self,
        trigger: Trigger,
    ) -> Result<(), TriggerStoreError>;

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError>;

    async fn get_trigger_by_name(
        &self,
        project: &ProjectId,
        name: &str,
    ) -> Result<Option<Trigger>, TriggerStoreError>;

    async fn find_trigger_id_for_name(
        &self,
        project: &ProjectId,
        name: &str,
    ) -> Result<Option<TriggerId>, TriggerStoreError>;

    async fn get_status(
        &self,
        project: &ProjectId,
        name: &str,
    ) -> Result<Option<Status>, TriggerStoreError>;

    async fn get_triggers_by_project(
        &self,
        project: &ProjectId,
        statuses: Option<Vec<Status>>,
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
}

#[async_trait]
impl TriggerStore for SqlTriggerStore {
    async fn install_trigger(
        &self,
        trigger: Trigger,
    ) -> Result<(), TriggerStoreError> {
        let active_model: triggers::ActiveModel = trigger.into();
        active_model.insert(&self.db.orm).await?;
        Ok(())
    }

    async fn update_trigger(
        &self,
        trigger: Trigger,
    ) -> Result<(), TriggerStoreError> {
        let project = trigger.project_id.clone();
        let active_model: triggers::ActiveModel = trigger.into();
        // Mark all the fields as dirty
        let active_model = active_model.reset_all();
        Triggers::update(active_model)
            .filter(triggers::Column::ProjectId.eq(project))
            .exec(&self.db.orm)
            .await?;
        Ok(())
    }

    async fn get_all_active_triggers(
        &self,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        let res = Triggers::find()
            .filter(
                triggers::Column::Status
                    .is_in([Status::Scheduled, Status::Paused]),
            )
            .all(&self.db.orm)
            .await?;
        Ok(res)
    }

    async fn get_triggers_by_project(
        &self,
        project: &ProjectId,
        statuses: Option<Vec<Status>>,
        before: Option<TriggerId>,
        after: Option<TriggerId>,
        limit: usize,
    ) -> Result<Vec<Trigger>, TriggerStoreError> {
        let mut query = Triggers::find()
            .filter(triggers::Column::ProjectId.eq(project.value()))
            .order_by_desc(triggers::Column::Id)
            .limit(Some(limit as u64));

        if let Some(statuses) = statuses {
            query = query.filter(triggers::Column::Status.is_in(statuses));
        }

        if let Some(before) = before {
            query = query.filter(triggers::Column::Id.gt(before.value()));
        }

        if let Some(after) = after {
            query = query.filter(triggers::Column::Id.lt(after.value()));
        }

        let res = query.all(&self.db.orm).await?;

        Ok(res)
    }

    async fn get_trigger_by_name(
        &self,
        project_id: &ProjectId,
        name: &str,
    ) -> Result<Option<Trigger>, TriggerStoreError> {
        let res =
            Triggers::find_by_id((name.to_string(), project_id.to_string()))
                .one(&self.db.orm)
                .await?;
        Ok(res)
    }

    async fn find_trigger_id_for_name(
        &self,
        project: &ProjectId,
        name: &str,
    ) -> Result<Option<TriggerId>, TriggerStoreError> {
        let res = Triggers::find_by_id((name.to_string(), project.to_string()))
            .select_only()
            .column(triggers::Column::Id)
            .into_tuple()
            .one(&self.db.orm)
            .await?;
        Ok(res)
    }

    async fn get_status(
        &self,
        project: &ProjectId,
        name: &str,
    ) -> Result<Option<Status>, TriggerStoreError> {
        let res: Option<Status> = Triggers::find()
            .select_only()
            .column(triggers::Column::Status)
            .filter(triggers::Column::Name.eq(name))
            .filter(triggers::Column::ProjectId.eq(project.to_string()))
            .into_tuple()
            .one(&self.db.orm)
            .await?;

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Timelike, Utc};

    use super::{SqlTriggerStore, TriggerStore};
    use crate::database::errors::DatabaseError;
    use crate::database::Database;
    use crate::model::ValidShardedId;
    use crate::types::{
        Action,
        ProjectId,
        Status,
        Trigger,
        TriggerId,
        Webhook,
    };

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

            project_id: project,
            name: name.to_string(),
            description: Some(format!("Desc: {}", name)),
            created_at: now,
            updated_at: None,
            payload: None,
            schedule: None,
            action: Action::Webhook(Webhook {
                url: "http://test".to_string(),
                http_method: crate::types::HttpMethod::Get,
                timeout_s: Duration::from_secs(5),
                retry: None,
            }),
            status,
            last_ran_at: None,
        }
    }

    #[tokio::test]
    async fn test_sql_trigger_store() -> anyhow::Result<()> {
        let db = Database::in_memory().await?;
        let store = SqlTriggerStore::new(db);

        let project1 = ProjectId::generate();
        let project2 = ProjectId::generate();

        let t1 = build_trigger("t1", project1.clone(), Status::Scheduled);
        let t2 = build_trigger("t2", project1.clone(), Status::Paused);
        let t3 = build_trigger("t3", project2.clone(), Status::Scheduled);
        let t4 = build_trigger("t4", project2.clone(), Status::Expired);

        // Test installs
        store.install_trigger(t1.clone()).await?;
        store.install_trigger(t2.clone()).await?;
        store.install_trigger(t3.clone()).await?;
        store.install_trigger(t4.clone()).await?;

        // Test getters
        assert_eq!(
            store.get_trigger_by_name(&project1, &t1.name).await?,
            Some(t1.clone())
        );
        assert_eq!(
            store.get_trigger_by_name(&project1, &t2.name).await?,
            Some(t2.clone())
        );
        assert_eq!(
            store.get_trigger_by_name(&project2, &t3.name).await?,
            Some(t3.clone())
        );
        assert_eq!(
            store.get_trigger_by_name(&project2, &t4.name).await?,
            Some(t4.clone())
        );
        // Wrong project.
        assert_eq!(store.get_trigger_by_name(&project1, &t4.name).await?, None);

        // Test fetching non existent trigger
        assert_eq!(
            store
                .get_trigger_by_name(&project1, &"non_existent".to_string())
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
            .get_triggers_by_project(&project1, None, None, None, 100)
            .await?;
        let mut expected = vec![t1.clone(), t2.clone()];
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        results.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(results, expected);

        // Test Get Status
        assert_eq!(
            Some(Status::Scheduled),
            store.get_status(&project1, &t1.name).await?
        );
        assert_eq!(
            Some(Status::Paused),
            store.get_status(&project1, &t2.name).await?
        );

        // Test get by status
        let results = store
            .get_triggers_by_project(
                &project1,
                Some(vec![Status::Paused]),
                None,
                None,
                100,
            )
            .await?;
        assert_eq!(results, vec![t2.clone()]);

        // Test update trigger
        let mut new_t1 = t1.clone();
        new_t1.status = Status::Expired;

        store.update_trigger(new_t1.clone()).await?;
        assert_eq!(
            store.get_trigger_by_name(&project1, &t1.name).await?,
            Some(new_t1.clone())
        );

        //
        let mut mismatch_project_t1 = new_t1.clone();
        mismatch_project_t1.project_id = ProjectId::generate();
        mismatch_project_t1.status = Status::Scheduled;
        assert!(matches!(
            store.update_trigger(mismatch_project_t1.clone()).await,
            Err(DatabaseError::DB(sea_orm::DbErr::RecordNotUpdated))
        ));
        assert_ne!(
            store.get_trigger_by_name(&project1, &t1.name).await?,
            Some(mismatch_project_t1)
        );

        Ok(())
    }
}
