use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub github_token: Option<String>,
    pub database_path: Option<String>,
    pub history: HistoryConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HistoryConfig {
    pub max_entries: usize,
    pub persist: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            persist: true,
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s =
            Config::builder().add_source(File::with_name("config/Settings").required(false));

        // Set defaults
        s = s.set_default("history.max_entries", 1000)?;
        s = s.set_default("history.persist", true)?;

        let config = s.build()?;
        config.try_deserialize()
    }
}
