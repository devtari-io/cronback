use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("database error: {0}")]
    QueryError(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    ParseError(#[from] serde_json::Error),
}
