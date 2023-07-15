use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Projects::Id).primary_key().string())
                    .col(
                        ColumnDef::new(Projects::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Projects::LastStatusChangedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Projects::Status)
                            .enumeration(
                                Projects::Status,
                                vec![
                                    ProjectStatus::Enabled,
                                    ProjectStatus::Disabled,
                                    ProjectStatus::QuotaExceeded,
                                    ProjectStatus::PendingDeletion,
                                ],
                            )
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Projects {
    Table,
    Id,
    CreatedAt,
    LastStatusChangedAt,
    Status,
}

#[derive(Iden)]
enum ProjectStatus {
    Enabled,
    Disabled,
    QuotaExceeded,
    PendingDeletion,
}
