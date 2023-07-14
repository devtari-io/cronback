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
                    .col(ColumnDef::new(Triggers::Project).string().not_null())
                    .col(ColumnDef::new(Triggers::Name).string().not_null())
                    .col(ColumnDef::new(Triggers::Description).string())
                    .col(
                        ColumnDef::new(Triggers::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Triggers::UpdatedAt).date_time())
                    .col(ColumnDef::new(Triggers::Reference).string())
                    .col(ColumnDef::new(Triggers::Payload).json())
                    .col(ColumnDef::new(Triggers::Schedule).json())
                    .col(ColumnDef::new(Triggers::Action).json().not_null())
                    .col(ColumnDef::new(Triggers::Status).string().not_null())
                    .col(ColumnDef::new(Triggers::LastRanAt).date_time())
                    .primary_key(
                        Index::create()
                            .col(Triggers::Id)
                            .col(Triggers::Project),
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
                    .col(Triggers::Project)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("UQ_triggers_project_reference")
                    .table(Triggers::Table)
                    .col(Triggers::Project)
                    .col(Triggers::Reference)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("IX_triggers_status")
                    .table(Triggers::Table)
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
    Project,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
    Reference,
    Payload,
    Schedule,
    Action,
    Status,
    LastRanAt,
}
