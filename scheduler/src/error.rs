use tonic::Status;

use crate::sched::triggers::TriggerError;

// implement From<HandlerError> for Status
impl From<TriggerError> for Status {
    fn from(e: TriggerError) -> Self {
        // match variants of TriggerError
        match e {
            | TriggerError::JoinError(e) => {
                Status::internal(format!("Internal error: {e}"))
            }
            | e => Status::invalid_argument(e.to_string()),
        }
    }
}
