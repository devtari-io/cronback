pub mod attempt_log_store;
mod errors;
pub mod models;
pub mod pagination;
pub mod run_store;

pub use errors::DatabaseError;
use migration::{Migrator, MigratorTrait};
use sea_orm::TransactionTrait;

#[derive(Clone)]
pub struct Database {
    pub orm: sea_orm::DatabaseConnection,
}

impl Database {
    pub async fn connect(conn_string: &str) -> Result<Self, sea_orm::DbErr> {
        Ok(Self {
            orm: sea_orm::Database::connect(conn_string).await?,
        })
    }

    pub async fn in_memory() -> Result<Self, sea_orm::DbErr> {
        let conn = Self::connect("sqlite::memory:").await?;
        conn.migrate().await?;
        Ok(conn)
    }

    pub async fn migrate(&self) -> Result<(), sea_orm::DbErr> {
        let conn = self.orm.begin().await?;
        Migrator::up(&conn, None).await?;
        conn.commit().await?;
        Ok(())
    }
}
