use async_trait::async_trait;
use sea_query::{ColumnDef, Expr, Iden, Index, Table};

use super::errors::DatabaseError;
use super::helpers::{
    get_by_id_query,
    insert_query,
    paginated_query,
    GeneratedJsonField,
    KVIden,
};
use crate::database::Database;
use crate::model::ModelId;
use crate::types::{ActionAttemptLog, AttemptLogId, ProjectId, RunId};

#[derive(Iden)]
enum AttemptsIden {
    Attempts,
    RunId,
}

pub type AttemptLogStoreError = DatabaseError;

#[async_trait]
pub trait AttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: &ActionAttemptLog,
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

    pub async fn prepare(&self) -> Result<(), AttemptLogStoreError> {
        let sql = Table::create()
            .table(AttemptsIden::Attempts)
            .if_not_exists()
            .col(ColumnDef::new(KVIden::Id).text().primary_key())
            .col(ColumnDef::new(KVIden::Value).json_binary())
            .col(
                ColumnDef::new(KVIden::Project)
                    .text()
                    .generate_from_json_field(KVIden::Value, "project"),
            )
            .col(
                ColumnDef::new(AttemptsIden::RunId)
                    .text()
                    .generate_from_json_field(KVIden::Value, "run"),
            )
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;

        // Create the indices
        let sql = Index::create()
            .if_not_exists()
            .name("IX_attempts_project")
            .table(AttemptsIden::Attempts)
            .col(KVIden::Project)
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;

        let sql = Index::create()
            .if_not_exists()
            .name("IX_attempts_runid")
            .table(AttemptsIden::Attempts)
            .col(AttemptsIden::RunId)
            .build_any(self.db.schema_builder().as_ref());
        sqlx::query(&sql).execute(&self.db.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl AttemptLogStore for SqlAttemptLogStore {
    async fn log_attempt(
        &self,
        attempt: &ActionAttemptLog,
    ) -> Result<(), AttemptLogStoreError> {
        insert_query(&self.db, AttemptsIden::Attempts, &attempt.id, attempt)
            .await
    }

    async fn get_attempts_for_run(
        &self,
        project: &ProjectId,
        id: &RunId,
        before: Option<AttemptLogId>,
        after: Option<AttemptLogId>,
        limit: usize,
    ) -> Result<Vec<ActionAttemptLog>, AttemptLogStoreError> {
        paginated_query(
            &self.db,
            AttemptsIden::Attempts,
            Expr::col(AttemptsIden::RunId)
                .eq(id.value())
                .and(Expr::col(KVIden::Project).eq(project.value())),
            &before,
            &after,
            Some(limit),
        )
        .await
    }

    async fn get_attempt(
        &self,
        project_id: &ProjectId,
        id: &AttemptLogId,
    ) -> Result<Option<ActionAttemptLog>, AttemptLogStoreError> {
        get_by_id_query(&self.db, AttemptsIden::Attempts, project_id, id).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Timelike, Utc};
    use chrono_tz::UTC;

    use super::{AttemptLogStore, SqlAttemptLogStore};
    use crate::database::Database;
    use crate::model::ValidShardedId;
    use crate::types::{
        ActionAttemptLog,
        AttemptLogId,
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
        let now = Utc::now().with_timezone(&UTC).with_nanosecond(0).unwrap();

        ActionAttemptLog {
            id: AttemptLogId::generate(project).into(),
            run: run_id.clone(),
            trigger: TriggerId::generate(project).into(),
            project: project.clone(),
            status: crate::types::AttemptStatus::Succeeded,
            details: crate::types::AttemptDetails::WebhookAttemptDetails(
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
        store.prepare().await?;

        let project = ProjectId::generate();
        let project2 = ProjectId::generate();
        let inv1 = RunId::generate(&project);
        let inv2 = RunId::generate(&project);

        let a1 = build_attempt(&project, &inv1);
        let a2 = build_attempt(&project, &inv2);
        let a3 = build_attempt(&project, &inv1);

        // Test log attempts
        store.log_attempt(&a1).await?;
        store.log_attempt(&a2).await?;
        store.log_attempt(&a3).await?;

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
