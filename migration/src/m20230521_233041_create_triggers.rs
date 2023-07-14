use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Triggers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Triggers::Id).string().not_null())
                    .col(ColumnDef::new(Triggers::Name).string().not_null())
                    .col(
                        ColumnDef::new(Triggers::ProjectId).string().not_null(),
                    )
                    .col(ColumnDef::new(Triggers::Description).string())
                    .col(
                        ColumnDef::new(Triggers::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Triggers::UpdatedAt).date_time())
                    .col(ColumnDef::new(Triggers::Payload).json())
                    .col(ColumnDef::new(Triggers::Schedule).json())
                    .col(ColumnDef::new(Triggers::Action).json().not_null())
                    .col(ColumnDef::new(Triggers::Status).string().not_null())
                    .col(ColumnDef::new(Triggers::LastRanAt).date_time())
                    .primary_key(
                        Index::create()
                            .col(Triggers::Name)
                            .col(Triggers::ProjectId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_triggers_project")
                    .table(Triggers::Table)
                    .col(Triggers::ProjectId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("UQ_triggers_project_trigger_name")
                    .table(Triggers::Table)
                    .col(Triggers::ProjectId)
                    .col(Triggers::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("UQ_triggers_project_trigger_id")
                    .table(Triggers::Table)
                    .col(Triggers::ProjectId)
                    .col(Triggers::Id)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_triggers_project_status")
                    .table(Triggers::Table)
                    .col(Triggers::ProjectId)
                    .col(Triggers::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Triggers::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Triggers {
    Table,
    Id,
    ProjectId,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
    Payload,
    Schedule,
    Action,
    Status,
    LastRanAt,
}
