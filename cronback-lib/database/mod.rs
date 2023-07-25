mod errors;
mod pagination;

pub use errors::DatabaseError;
pub use pagination::*;

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
