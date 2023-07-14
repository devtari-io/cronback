use sqlx::postgres::PgDatabaseError;
use sqlx::sqlite::SqliteError;
use thiserror::Error;

const SQLITE_UNIQUE_CONSTRAINT_FAILED_CODE: &str = "2067";
const POSTGRES_UNIQUE_CONSTRAINT_FAILED_CODE: &str = "23505";

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("serialization error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("constraint error: violated unique constraint")]
    DuplicateRecord,

    #[error("database error: {0}")]
    DB(sea_orm::DbErr),
}

impl From<sea_orm::DbErr> for DatabaseError {
    fn from(value: sea_orm::DbErr) -> Self {
        match value {
            | sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(
                sqlx::Error::Database(db_err),
            )) if is_duplicate_record_error(db_err.as_ref()) => {
                DatabaseError::DuplicateRecord
            }
            | _ => DatabaseError::DB(value),
        }
    }
}

fn is_duplicate_record_error(db_err: &dyn sqlx::error::DatabaseError) -> bool {
    if db_err.try_downcast_ref::<SqliteError>().is_some() {
        return db_err.code().map(|a| a.to_string())
            == Some(SQLITE_UNIQUE_CONSTRAINT_FAILED_CODE.to_string());
    }
    if db_err.try_downcast_ref::<PgDatabaseError>().is_some() {
        return db_err.code().map(|a| a.to_string())
            == Some(POSTGRES_UNIQUE_CONSTRAINT_FAILED_CODE.to_string());
    }
    false
}
