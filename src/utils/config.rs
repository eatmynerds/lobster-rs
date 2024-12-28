use crate::{Languages, Provider};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub use_external_menu: bool,
    pub download: String,
    pub provider: Provider,
    pub subs_language: Languages,
    pub player: String,
    pub history: bool,
    pub histfile: String,
    pub image_preview: bool,
    pub debug: bool,
}

impl Config {
    pub fn new() -> Self {
        Config {
            player: String::from("mpv"),
            download: String::from(
                std::env::current_dir()
                    .expect("Failed to get current dir")
                    .to_str()
                    .expect("Failed to convert path to str"),
            ),
            provider: Provider::Vidcloud,
            history: false,
            subs_language: Languages::English,
            histfile: String::from("~/.local/share/lobster/lobster_history.txt"),
            use_external_menu: false,
            image_preview: false,
            debug: false,
        }
    }

    pub fn load_from_file(file_path: &Path) -> anyhow::Result<Self> {
        let home_dir = dirs::home_dir().context("Failed to retrieve the home directory")?;

        let config_file_path = home_dir.join(".config/lobster_rs/config.toml");
        if !config_file_path.exists() {
            let default_config = Config::new();
            let content = toml::to_string(&default_config)
                .with_context(|| "Failed to serialize the default config")?;

            if let Some(parent) = config_file_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
            }

            let mut file = File::create(&config_file_path)
                .with_context(|| format!("Failed to create config file: {:?}", config_file_path))?;

            file.write_all(content.as_bytes()).with_context(|| {
                format!("Failed to write to config file: {:?}", config_file_path)
            })?;

            return Ok(default_config);
        }

        let content = std::fs::read_to_string(&config_file_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_file_path))?;
        toml::from_str(&content).context("Failed to parse config.toml")
    }
}
