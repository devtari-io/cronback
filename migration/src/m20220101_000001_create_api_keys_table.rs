use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .col(
                        ColumnDef::new(ApiKeys::KeyId)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ApiKeys::Hash).text().not_null())
                    .col(ColumnDef::new(ApiKeys::HashVersion).text().not_null())
                    .col(ColumnDef::new(ApiKeys::ProjectId).text().not_null())
                    .col(ColumnDef::new(ApiKeys::Name).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum ApiKeys {
    Table,
    KeyId,
    Hash,
    HashVersion,
    ProjectId,
    Name,
}
