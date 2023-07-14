pub mod attempt_log_store;
mod errors;
mod helpers;
pub mod invocation_store;
pub mod trigger_store;

use sea_query::{
    PostgresQueryBuilder,
    QueryBuilder,
    SchemaBuilder,
    SqliteQueryBuilder,
};
use sqlx::AnyPool;

#[derive(Clone)]
pub struct Database {
    pub pool: AnyPool,
}

impl Database {
    pub async fn connect(conn_string: &str) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: AnyPool::connect(conn_string).await?,
        })
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        Self::connect("sqlite::memory:").await
    }

    pub fn builder(&self) -> Box<dyn QueryBuilder> {
        match self.pool.any_kind() {
            | sqlx::any::AnyKind::Postgres => Box::new(PostgresQueryBuilder),
            | sqlx::any::AnyKind::Sqlite => Box::new(SqliteQueryBuilder),
        }
    }

    pub fn schema_builder(&self) -> Box<dyn SchemaBuilder> {
        match self.pool.any_kind() {
            | sqlx::any::AnyKind::Postgres => Box::new(PostgresQueryBuilder),
            | sqlx::any::AnyKind::Sqlite => Box::new(SqliteQueryBuilder),
        }
    }
}
