use dto_helpers::IntoProto;
use proto::scheduler_proto;
use serde::{Deserialize, Serialize};

#[derive(IntoProto, Debug, Deserialize, Serialize, Clone, Default)]
#[into_proto(into = "scheduler_proto::RunMode")]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunMode {
    Sync,
    #[default]
    Async,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub(crate) struct RunTrigger {
    pub mode: RunMode,
}
