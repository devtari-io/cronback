//! Configuration Model

use std::collections::HashSet;

use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(unused)]
pub enum Role {
    Api,
    Dispatcher,
    Scheduler,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DispatcherConfig {}

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerConfig {}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
///
///
/// * `roles`: Which roles the binary will start with
/// * `api`: Configuration of the API server
/// * `dispatcher`:  Configuration of the dispatcher
/// * `scheduler`:  Configuration of the scheduler
pub struct CoreConfig {
    pub roles: HashSet<Role>,
    pub api: ApiConfig,
    pub dispatcher: DispatcherConfig,
    pub scheduler: SchedulerConfig,
}

impl CoreConfig {
    /// Loads a configuration file from path, overlayed on top of a set of defaults
    ///
    /// * `config_file`: The path of the configuration file to load.
    pub fn from_path(path: &Option<String>) -> Result<CoreConfig, ConfigError> {
        // Load config file
        // TODO: Figure out a better way to load defaults
        let mut config = Config::builder().add_source(File::with_name("./config/default.toml"));
        if let Some(path) = path {
            config = config.add_source(File::with_name(path));
        }
        Self::parse_config(config.build()?)
    }

    fn parse_config(config: Config) -> Result<CoreConfig, ConfigError> {
        config.try_deserialize()
    }
}
