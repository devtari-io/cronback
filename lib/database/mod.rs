pub mod attempt_log_store;
mod errors;
pub mod models;
pub mod pagination;
pub mod run_store;
pub mod trigger_store;

use migration::{Migrator, MigratorTrait};

#[derive(Clone)]
pub struct Database {
    pub orm: sea_orm::DatabaseConnection,
}

impl Database {
    pub async fn connect(conn_string: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            orm: sea_orm::Database::connect(conn_string).await?,
        })
    }

    pub async fn in_memory() -> Result<Self, anyhow::Error> {
        let conn = Self::connect("sqlite::memory:").await?;
        Migrator::up(&conn.orm, None).await?;
        Ok(conn)
    }
}
