#![cfg(feature = "validation")]
use validator::ValidationError;

pub fn validation_error(
    code: &'static str,
    message: String,
) -> ValidationError {
    let mut validation_e = ValidationError::new(code);
    validation_e.message = Some(message.into());
    validation_e
}
