use std::collections::HashSet;

use sea_orm::ConnectOptions;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiSvcConfig {
    pub address: String,
    pub port: u16,
    pub database_uri: String,
    pub admin_api_keys: HashSet<String>,
    pub log_request_body: bool,
    pub log_response_body: bool,
}

impl From<ApiSvcConfig> for ConnectOptions {
    fn from(value: ApiSvcConfig) -> Self {
        value.database_uri.into()
    }
}
