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
                    .col(ColumnDef::new(Attempts::Run).string().not_null())
                    .col(ColumnDef::new(Attempts::Trigger).string().not_null())
                    .col(ColumnDef::new(Attempts::Project).string().not_null())
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
                        ColumnDef::new(Attempts::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(Attempts::Id)
                            .col(Attempts::Project),
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
                    .col(Attempts::Project)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_attempts_runid")
                    .table(Attempts::Table)
                    .col(Attempts::Run)
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
    Run,
    Trigger,
    Project,
    Status,
    Details,
    CreatedAt,
}

#[derive(Iden)]
enum AttemptStatus {
    Succeeded,
    Failed,
}
