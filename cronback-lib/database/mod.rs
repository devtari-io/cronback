mod errors;
mod pagination;

use async_trait::async_trait;
pub use errors::DatabaseError;
pub use pagination::*;

#[async_trait]
pub trait DbMigrator {
    async fn migrate_up(&self) -> Result<(), DatabaseError>;
}

#[derive(Clone)]
pub struct Database {
    pub orm: sea_orm::DatabaseConnection,
}

impl Database {
    pub async fn connect(conn_string: &str) -> Result<Self, DatabaseError> {
        Ok(Self {
            orm: sea_orm::Database::connect(conn_string).await?,
        })
    }

    pub async fn in_memory() -> Result<Self, DatabaseError> {
        Self::connect("sqlite::memory:").await
    }
}
