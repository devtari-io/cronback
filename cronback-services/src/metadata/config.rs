use sea_orm::ConnectOptions;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataSvcConfig {
    pub cell_id: u32,
    pub address: String,
    pub port: u16,
    pub request_processing_timeout_s: u64,
    pub database_uri: String,
}

impl From<MetadataSvcConfig> for ConnectOptions {
    fn from(value: MetadataSvcConfig) -> Self {
        value.database_uri.into()
    }
}
