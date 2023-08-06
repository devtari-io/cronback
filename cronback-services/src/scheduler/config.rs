use sea_orm::ConnectOptions;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerSvcConfig {
    // Cell Id of the current scheduler
    pub cell_id: u32,
    pub address: String,
    pub port: u16,
    pub request_processing_timeout_s: u64,
    pub spinner_yield_max_ms: u64,
    pub max_triggers_per_tick: u64,
    pub database_uri: String,
    pub db_flush_s: u64,
    pub dangerous_fast_forward: bool,
}

impl From<SchedulerSvcConfig> for ConnectOptions {
    fn from(value: SchedulerSvcConfig) -> Self {
        value.database_uri.into()
    }
}
