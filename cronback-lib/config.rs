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

#[derive(Debug, Clone, Deserialize)]
pub struct MainConfig {
    pub roles: HashSet<String>,
    pub prometheus_address: String,
    pub prometheus_port: u16,
    pub dispatcher_cell_map: HashMap<u64, String>,
    pub scheduler_cell_map: HashMap<u64, String>,
    pub metadata_cell_map: HashMap<u64, String>,
}

#[derive(Debug)]
pub struct ConfigLoader {
    builder: ConfigBuilder<DefaultState>,
}

impl ConfigLoader {
    /// Loads a fresh copy of the configuration from source.
    pub fn load_main(&self) -> Result<MainConfig, ConfigError> {
        let c = self.builder.build_cloned()?;
        c.get("main")
    }

    /// Loads a fresh copy of a specific configuration section from source.
    pub fn load_section<'de, C>(&self, section: &str) -> Result<C, ConfigError>
    where
        C: Deserialize<'de>,
    {
        let c = self.builder.build_cloned()?;
        c.get(section)
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
}
