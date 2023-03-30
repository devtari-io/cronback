pub mod attempt_log_store;
mod helpers;
pub mod invocation_store;
pub mod trigger_store;

use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SqliteDatabase {
    pub pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn connect(conn_string: &str) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: SqlitePool::connect(conn_string).await?,
        })
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        Self::connect("sqlite::memory:").await
    }
}