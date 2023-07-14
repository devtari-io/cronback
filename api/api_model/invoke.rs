use proto::scheduler_proto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvocationMode {
    Sync,
    #[default]
    Async,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct InvokeTrigger {
    pub mode: InvocationMode,
}

impl From<InvocationMode> for i32 {
    fn from(value: InvocationMode) -> Self {
        let enum_value = match value {
            | InvocationMode::Sync => scheduler_proto::InvocationMode::Sync,
            | InvocationMode::Async => scheduler_proto::InvocationMode::Async,
        };
        enum_value as i32
    }
}
