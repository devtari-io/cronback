use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Attempts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Attempts::Id).string().not_null())
                    .col(ColumnDef::new(Attempts::RunId).string().not_null())
                    .col(
                        ColumnDef::new(Attempts::TriggerId).string().not_null(),
                    )
                    .col(
                        ColumnDef::new(Attempts::ProjectId).string().not_null(),
                    )
                    .col(
                        ColumnDef::new(Attempts::Status)
                            .enumeration(
                                Attempts::Status,
                                vec![
                                    AttemptStatus::Succeeded,
                                    AttemptStatus::Failed,
                                ],
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(Attempts::Details).json().not_null())
                    .col(
                        ColumnDef::new(Attempts::AttemptNum)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Attempts::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(Attempts::Id)
                            .col(Attempts::ProjectId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_attempts_project")
                    .table(Attempts::Table)
                    .col(Attempts::ProjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_attempts_runid")
                    .table(Attempts::Table)
                    .col(Attempts::RunId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Attempts::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Attempts {
    Table,
    Id,
    RunId,
    TriggerId,
    ProjectId,
    Status,
    Details,
    AttemptNum,
    CreatedAt,
}

#[derive(Iden)]
enum AttemptStatus {
    Succeeded,
    Failed,
}
