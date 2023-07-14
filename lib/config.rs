//! Configuration Model

use std::collections::{HashMap, HashSet};

use config::builder::DefaultState;
use config::{
    Config as ConfigRaw,
    ConfigBuilder,
    ConfigError,
    Environment,
    File,
    FileFormat,
};
use serde::Deserialize;
use valuable::Valuable;

#[derive(Debug, Valuable, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(unused)]
pub enum Role {
    Api,
    Dispatcher,
    Scheduler,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MainConfig {
    pub roles: HashSet<Role>,
    pub prometheus_address: String,
    pub prometheus_port: u16,
    pub dispatcher_cell_map: HashMap<u64, String>,
    pub scheduler_cell_map: HashMap<u64, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DispatcherConfig {
    pub address: String,
    pub port: u16,
    pub request_processing_timeout_s: u64,
    pub database_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerConfig {
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

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub address: String,
    pub port: u16,
    pub database_uri: String,
    pub admin_api_keys: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
///
///
/// * `roles`: Which roles the binary will start with
/// * `api`: Configuration of the API server
/// * `dispatcher`:  Configuration of the dispatcher
/// * `scheduler`:  Configuration of the scheduler
pub struct Config {
    pub main: MainConfig,
    pub api: ApiConfig,
    pub dispatcher: DispatcherConfig,
    pub scheduler: SchedulerConfig,
}

#[derive(Debug)]
pub struct ConfigLoader {
    builder: ConfigBuilder<DefaultState>,
}

impl ConfigLoader {
    /// Loads a fresh copy of the configuration from source.
    pub fn load(&self) -> Result<Config, ConfigError> {
        Self::deserialize(self.builder.build_cloned()?)
    }

    /// creates a new loader configured to load the default and overlays
    /// the user supplied config (if supplied).
    ///
    /// * `config_file`: The path of the configuration file to load.
    pub fn from_path(path: &Option<String>) -> ConfigLoader {
        let raw = include_str!("default.toml");
        let mut builder = ConfigRaw::builder()
            .add_source(File::from_str(raw, FileFormat::Toml))
            .add_source(
                Environment::with_prefix("CRONBACK")
                    .try_parsing(true)
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("api.admin_api_keys"),
            );
        if let Some(path) = path {
            builder = builder.add_source(File::with_name(path));
        }
        ConfigLoader { builder }
    }

    fn deserialize(config: ConfigRaw) -> Result<Config, ConfigError> {
        config.try_deserialize()
    }
}
