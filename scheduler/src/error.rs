use lib::types::TriggerId;
use thiserror::Error;

use crate::db_model::triggers::Status;
use crate::sched::event_dispatcher::DispatchError;
use crate::trigger_store::TriggerStoreError;

#[allow(unused)]
#[derive(Error, Debug)]
pub enum TriggerError {
    #[error("Cannot parse cron expression")]
    CronParse(#[from] cron::error::Error),
    #[error(
        "Unrecognized timezone '{0}' was supplied, are you sure this is an \
         IANA timezone?"
    )]
    InvalidTimezone(String),
    #[error("Trigger '{0}' has no schedule!")]
    NotScheduled(TriggerId),
    #[error("Trigger '{0}' is unknown to this scheduler!")]
    NotFound(String),
    #[error("Cannot {0} on a trigger with status {1}")]
    InvalidStatus(String, Status),
    //join error
    #[error("Internal async processing failure!")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Operation on underlying trigger store failed: {0}")]
    TriggerStore(#[from] TriggerStoreError),
    #[error("Cannot dispatch a run for this trigger")]
    Run(#[from] DispatchError),
    #[error("Trigger '{0}' already exists")]
    AlreadyExists(/* name */ String),
    #[error("{0}")]
    PreconditionFailed(String),
}

// implement From<HandlerError> for Status
impl From<TriggerError> for tonic::Status {
    fn from(e: TriggerError) -> Self {
        // match variants of TriggerError
        match e {
            | TriggerError::InvalidStatus { .. } => {
                tonic::Status::failed_precondition(e.to_string())
            }
            | TriggerError::JoinError(e) => {
                tonic::Status::internal(format!("Internal error: {e}"))
            }
            | TriggerError::NotFound(e) => tonic::Status::not_found(e),
            | e @ TriggerError::AlreadyExists(_) => {
                tonic::Status::already_exists(e.to_string())
            }
            | TriggerError::PreconditionFailed(e) => {
                tonic::Status::failed_precondition(e)
            }
            | e => tonic::Status::invalid_argument(e.to_string()),
        }
    }
}
