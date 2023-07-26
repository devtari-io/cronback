mod errors;
mod pagination;

use async_trait::async_trait;
pub use errors::DatabaseError;
pub use pagination::*;
use sea_orm::{ConnectOptions, TransactionTrait};
use sea_orm_migration::MigratorTrait;

#[derive(Clone)]
pub struct Database {
    pub orm: sea_orm::DatabaseConnection,
}

impl Database {
    pub async fn connect<O, M>(opts: O) -> Result<Self, DatabaseError>
    where
        M: DbMigration,
        O: Into<ConnectOptions>,
    {
        let orm = sea_orm::Database::connect(opts).await?;
        //migrate
        M::up(&orm).await?;
        Ok(Self { orm })
    }
}

#[async_trait]
pub trait DbMigration: Sync + 'static {
    async fn up(db: &sea_orm::DatabaseConnection) -> Result<(), DatabaseError>;
    async fn down(
        db: &sea_orm::DatabaseConnection,
    ) -> Result<(), DatabaseError>;
}

// Performs no migrations.
pub struct NoMigration;

#[async_trait]
impl DbMigration for NoMigration {
    async fn up(
        _db: &sea_orm::DatabaseConnection,
    ) -> Result<(), DatabaseError> {
        Ok(())
    }

    async fn down(
        _db: &sea_orm::DatabaseConnection,
    ) -> Result<(), DatabaseError> {
        Ok(())
    }
}

#[async_trait]
impl<T> DbMigration for T
where
    T: MigratorTrait + Sync + 'static,
{
    async fn up(db: &sea_orm::DatabaseConnection) -> Result<(), DatabaseError> {
        let conn = db.begin().await?;
        <Self as MigratorTrait>::up(&conn, None).await?;
        conn.commit().await?;
        Ok(())
    }

    async fn down(
        db: &sea_orm::DatabaseConnection,
    ) -> Result<(), DatabaseError> {
        let conn = db.begin().await?;
        <Self as MigratorTrait>::down(&conn, None).await?;
        conn.commit().await?;
        Ok(())
    }
}
