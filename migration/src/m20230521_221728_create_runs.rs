use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Runs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Runs::Id).string().not_null())
                    .col(ColumnDef::new(Runs::TriggerId).string().not_null())
                    .col(ColumnDef::new(Runs::ProjectId).string().not_null())
                    .col(ColumnDef::new(Runs::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Runs::Payload).json())
                    .col(ColumnDef::new(Runs::Action).json().not_null())
                    .col(
                        ColumnDef::new(Runs::Status)
                            .enumeration(
                                Runs::Status,
                                vec![
                                    RunStatus::Attempting,
                                    RunStatus::Failed,
                                    RunStatus::Succeeded,
                                ],
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(Runs::LatestAttempt).json())
                    .primary_key(
                        Index::create().col(Runs::Id).col(Runs::ProjectId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_runs_project")
                    .table(Runs::Table)
                    .col(Runs::ProjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_runs_triggerid")
                    .table(Runs::Table)
                    .col(Runs::TriggerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_runs_status")
                    .table(Runs::Table)
                    .col(Runs::Status)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Runs::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Runs {
    Table,
    Id,
    TriggerId,
    ProjectId,
    CreatedAt,
    Payload,
    Action,
    Status,
    LatestAttempt,
}

#[derive(Iden)]
enum RunStatus {
    Attempting,
    Succeeded,
    Failed,
}
