use tonic::Status;

use crate::sched::triggers::TriggerError;

// implement From<HandlerError> for Status
impl From<TriggerError> for Status {
    fn from(e: TriggerError) -> Self {
        // match variants of TriggerError
        match e {
            | TriggerError::InvalidStatus { .. } => {
                Status::failed_precondition(e.to_string())
            }
            | TriggerError::JoinError(e) => {
                Status::internal(format!("Internal error: {e}"))
            }
            | TriggerError::NotFound(e) => Status::not_found(e),
            | e @ TriggerError::AlreadyExists(_) => {
                Status::already_exists(e.to_string())
            }
            | TriggerError::PreconditionFailed(e) => {
                Status::failed_precondition(e.to_string())
            }
            | e => Status::invalid_argument(e.to_string()),
        }
    }
}
