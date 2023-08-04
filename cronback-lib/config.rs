//! Configuration Model

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use config::builder::DefaultState;
use config::{
    Config as InnerConfig,
    ConfigBuilder as InnerBuilder,
    ConfigError as InnerConfigError,
    Environment,
    File,
    FileFormat,
};
use notify_debouncer_mini::{
    new_debouncer,
    DebouncedEvent,
    DebouncedEventKind,
};
use serde::Deserialize;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::prelude::CronbackService;
use crate::Shutdown;

type SectionMap = HashMap<String, Box<dyn Any + Send + Sync>>;
type SectionLoaders = HashMap<
    String,
    Arc<
        dyn Fn(
                &str,
                &InnerConfig,
            )
                -> Result<Box<dyn Any + Send + Sync>, InnerConfigError>
            + Send
            + Sync,
    >,
>;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("configuration error for service '{0}'")]
    ServiceConfigLoadError(String, #[source] InnerConfigError),

    #[error(transparent)]
    ConfigLoadError(#[from] InnerConfigError),
}

#[derive(Debug, Clone, Deserialize)]
pub struct MainConfig {
    pub roles: HashSet<String>,
    pub prometheus_address: String,
    pub prometheus_port: u16,
    pub dispatcher_cell_map: HashMap<u64, String>,
    pub scheduler_cell_map: HashMap<u64, String>,
    pub metadata_cell_map: HashMap<u64, String>,
}

#[derive(Clone)]
pub struct ConfigBuilder {
    env_prefix: String,
    builder: InnerBuilder<DefaultState>,
    file_sources: Vec<PathBuf>,
    section_loaders: SectionLoaders,
}

impl ConfigBuilder {
    /// creates a new builder configured to load the default from a toml string
    pub fn new(env_prefix: impl Into<String>, default_toml_src: &str) -> Self {
        let builder = InnerConfig::builder()
            .add_source(File::from_str(default_toml_src, FileFormat::Toml));
        let mut loader = ConfigBuilder {
            env_prefix: env_prefix.into(),
            builder,
            file_sources: Vec::default(),
            section_loaders: HashMap::default(),
        };
        // Register main section loader by default.
        loader = loader.register_section_loader::<MainConfig>("main");
        loader
    }

    /// Adds a static toml configuration string as a config source to the
    /// builder
    pub fn add_default_toml(self, raw: &str) -> Self {
        Self {
            builder: self
                .builder
                .add_source(File::from_str(raw, FileFormat::Toml)),
            ..self
        }
    }

    pub fn add_file_source(mut self, path: impl Into<PathBuf>) -> Self {
        let path: PathBuf = path.into();
        self.file_sources.push(path.clone());
        Self {
            builder: self.builder.add_source(File::from(path)),
            ..self
        }
    }

    pub fn register_service<S>(self) -> Self
    where
        S: CronbackService,
    {
        self.add_default_toml(S::DEFAULT_CONFIG_TOML)
            .register_section_loader::<<S as CronbackService>::ServiceConfig>(
            S::CONFIG_SECTION,
        )
    }

    // This needs to be the last source added to the builder to ensure that
    // environemnt variables are not overridden by files or default config.
    fn add_env_source(mut self) -> Self {
        self.builder = self.builder.add_source(
            Environment::with_prefix(&self.env_prefix)
                .try_parsing(true)
                .separator("__")
                .list_separator(","),
        );
        self
    }

    // Sets an override that will always be applied on top of all sources. This
    // is useful in testing scenarios where you want to override a value
    // that is normally set in the default config.
    pub fn set_override<K, V>(
        mut self,
        key: K,
        value: V,
    ) -> Result<Self, ConfigError>
    where
        K: AsRef<str>,
        V: Into<config::Value>,
    {
        self.builder = self.builder.set_override(key, value)?;
        Ok(self)
    }

    /// Instantiate the config without watching the configuration for changes
    pub fn build_once(mut self) -> Result<Config, ConfigError> {
        self = self.add_env_source();
        Config::from_builder(self, None)
    }

    pub fn build_and_watch(
        mut self,
        shutdown: Shutdown,
    ) -> Result<Config, ConfigError> {
        self = self.add_env_source();
        Config::from_builder(self, Some(shutdown))
    }

    /// Registers a function that teacher the builder how to load sections for
    /// registered services.
    pub fn register_section_loader<'de, C>(mut self, section: &str) -> Self
    where
        C: Deserialize<'de> + Send + Sync + 'static,
    {
        self.section_loaders.insert(
            section.to_owned(),
            Arc::new(move |section: &str, inner: &InnerConfig| {
                let section = inner.get::<C>(section)?;
                let section = Box::new(section) as Box<dyn Any + Send + Sync>;
                Ok(section)
            }),
        );
        self
    }
}

impl Default for ConfigBuilder {
    /// creates a new builder configured to load the default from a toml string
    fn default() -> Self {
        Self::new("CRONBACK", include_str!("main.toml"))
    }
}

struct CachedConfig {
    section_map: SectionMap,
    last_modified: Instant,
}

impl CachedConfig {
    fn new(section_map: SectionMap) -> Self {
        Self {
            section_map,
            last_modified: Instant::now(),
        }
    }
}

struct ConfigWatcher {
    builder: InnerBuilder<DefaultState>,
    section_loaders: SectionLoaders,
    files: Vec<PathBuf>,
}

impl ConfigWatcher {
    fn new(
        builder: InnerBuilder<DefaultState>,
        section_loaders: SectionLoaders,
        files: Vec<PathBuf>,
    ) -> Self {
        Self {
            builder,
            section_loaders,
            files,
        }
    }

    fn start_watching(
        self,
        cache: Arc<RwLock<CachedConfig>>,
        mut shutdown: Shutdown,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        // Automatically select the best implementation for watching files on
        // the current platform.
        let mut debouncer =
            new_debouncer(Duration::from_secs(2), None, tx).unwrap();
        for source in &self.files {
            info!("Installing watcher for file changes: {}", source.display());
            debouncer
                .watcher()
                .watch(source, notify::RecursiveMode::NonRecursive)
                .expect("watch files with notify");
        }
        std::thread::Builder::new()
            .name("config-watcher".to_owned())
            .spawn(move || {
                // It's important that we capture the watcher in the thread,
                // otherwise it'll be dropped and we won't be watching anything!
                let _debouncer = debouncer;
                info!("Configuration watcher thread has started");
                while !shutdown.is_shutdown() {
                    match rx.recv() {
                        | Ok(evs) => {
                            self.handle_events(evs, &cache);
                        }
                        | Err(e) => {
                            error!(
                                "Cannot continue watching configuration \
                                 changes: '{}', system will shutdown to avoid \
                                 crash-looping!",
                                e
                            );
                            shutdown.broadcast_shutdown();
                        }
                    }
                }
                info!("Config watcher thread has terminated");
            })
            .unwrap();
    }

    fn load_sections(&self) -> Result<SectionMap, ConfigError> {
        // load the config
        let inner = self.builder.build_cloned()?;
        let mut section_map = HashMap::default();

        for (section, loader) in &self.section_loaders {
            debug!("--> Loading configuration section {section}");
            let section_cfg = loader(section, &inner).map_err(|e| {
                ConfigError::ServiceConfigLoadError(section.to_owned(), e)
            })?;
            section_map.insert(section.to_owned(), section_cfg);
        }
        Ok(section_map)
    }

    fn handle_events(
        &self,
        evs: Result<Vec<DebouncedEvent>, Vec<notify::Error>>,
        cache: &Arc<RwLock<CachedConfig>>,
    ) {
        match evs {
            | Ok(evs) => {
                let mut should_update = false;
                for event in evs
                    .into_iter()
                    .filter(|e| e.kind == DebouncedEventKind::Any)
                {
                    should_update = true;
                    info!(
                        "Detected configuration file changes: {:?}",
                        event.path
                    );
                }
                if should_update {
                    self.try_update_cache(cache).unwrap_or_else(|e| {
                        error!(
                            "Error updating configuration, we will keep the  \
                             last configuration in memory a valid \
                             configuration is loaded: {}",
                            e
                        );
                    });
                }
            }
            | Err(e) => {
                warn!(
                    "Error watching configuration file, file changes might \
                     not be observed, but the system will continue to operate \
                     with the last known configuration: {}",
                    e.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                );
            }
        }
    }

    fn try_update_cache(
        &self,
        cache: &Arc<RwLock<CachedConfig>>,
    ) -> Result<(), ConfigError> {
        let section_map = self.load_sections()?;
        let mut cache = cache.write().unwrap();
        cache.section_map = section_map;
        info!(
            "Configuration has been updated. Last update was {:?} ago.",
            cache.last_modified.elapsed()
        );
        cache.last_modified = Instant::now();
        Ok(())
    }
}

/// Automatically watches sources and update the cache as needed.
/// If configuration loading has failed after the first initialisation (due to a
/// watched config file being invalid) the last loaded configuration will remain
/// loaded in cache and we will log the problem in WARN level to alert the user.
///
/// Safe to clone, all clones share the same cache and watcher infrastructure.
#[derive(Clone)]
pub struct Config {
    cache: Arc<RwLock<CachedConfig>>,
}

impl Config {
    fn from_builder(
        builder: ConfigBuilder,
        shutdown: Option<Shutdown>,
    ) -> Result<Self, ConfigError> {
        let loader = ConfigWatcher::new(
            builder.builder,
            builder.section_loaders,
            builder.file_sources,
        );

        let section_map = loader.load_sections()?;
        let cache = Arc::new(RwLock::new(CachedConfig::new(section_map)));

        if let Some(shutdown) = shutdown {
            // Start the config watcher thread
            loader.start_watching(cache.clone(), shutdown);
        }
        Ok(Self { cache })
    }

    /// Convenience function to get the main config
    pub fn get_main(&self) -> MainConfig {
        self.get("main")
    }

    /// Do not use this method directly, this will panic if the section doesn't
    /// exist, or of the type is not correct. Instead, use helpers in
    /// ServiceContext instead.
    pub fn get<C>(&self, section: &str) -> C
    where
        C: for<'de> Deserialize<'de> + Clone + 'static,
    {
        // We assume that validation happened at loading time and that the
        // section exists and valid. This function will panic if the section
        // doesn't exist in the config.
        self.cache
            .read()
            .unwrap()
            .section_map
            .get(section)
            // downcast_ref returns a reference to the boxed value if it is of
            // type T.
            .and_then(|boxed| {
                (&**boxed as &(dyn Any + 'static)).downcast_ref::<C>()
            })
            .unwrap()
            .clone()
    }

    pub fn last_modified(&self) -> Instant {
        self.cache.read().unwrap().last_modified
    }
}

const _: () = {
    const fn _assert_send<T: Send>() {}
    const fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<Config>();
    _assert_send::<CachedConfig>();
};
