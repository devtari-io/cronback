use dto_helpers::IntoProto;
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};

#[derive(IntoProto, Debug, Deserialize, Serialize, Clone, Default)]
#[into_proto(into = "scheduler_proto::InvocationMode")]
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
