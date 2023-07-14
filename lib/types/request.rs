use derive_more::{Display, From, Into};
use ulid::Ulid;

/// A debug key is a random string that is used to identify a request.
#[derive(
    Debug, Clone, Default, Eq, PartialEq, PartialOrd, Ord, Display, From, Into,
)]
pub struct RequestId(String);

impl RequestId {
    // generate random debug key
    pub fn new() -> Self {
        let key = Ulid::new().to_string();
        Self(key)
    }

    pub fn from(value: String) -> Self {
        Self(value)
    }
}
