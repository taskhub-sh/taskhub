use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub github_token: Option<String>,
    pub database_path: Option<String>,
    // Add other settings here as needed
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("config/Settings"))
            .build()?;
        s.try_deserialize()
    }
}
