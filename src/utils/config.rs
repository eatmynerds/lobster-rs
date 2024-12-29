use crate::{Args, Languages, Provider};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

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
            histfile: format!(
                "{}/lobster/lobster_history.txt",
                dirs::config_dir()
                    .expect("Faield to get config dir")
                    .display()
            ),
            use_external_menu: false,
            image_preview: false,
            debug: false,
        }
    }

    pub fn load_config() -> anyhow::Result<Self> {
        let config_dir = dirs::config_dir().context("Failed to retrieve the config directory")?;

        let config = Config::load_from_file(Path::new(&format!(
            "{}/lobster/config.toml",
            config_dir.display()
        )))?;

        Ok(config)
    }

    pub fn load_from_file(file_path: &Path) -> anyhow::Result<Self> {
        if !file_path.exists() {
            let default_config = Config::new();
            let content = toml::to_string(&default_config)
                .with_context(|| "Failed to serialize the default config")?;

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
            }

            let mut file = File::create(&file_path)
                .with_context(|| format!("Failed to create config file: {:?}", file_path))?;

            file.write_all(content.as_bytes())
                .with_context(|| format!("Failed to write to config file: {:?}", file_path))?;

            return Ok(default_config);
        }

        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read config file: {:?}", file_path))?;
        toml::from_str(&content).context("Failed to parse config.toml")
    }

    pub fn program_configuration<'a>(args: &'a mut Args, config: &'a mut Self) -> &'a mut Args {
        if cfg!(target_os = "linux") {
            args.rofi = if !args.rofi {
                config.use_external_menu
            } else {
                args.rofi
            };
        } else {
            args.rofi = false;
        }

        args.download = Some(
            match &args.download {
                Some(download) => download.as_str(),
                None => &config.download,
            }
            .to_string(),
        );

        args.provider = Some(match &args.provider {
            Some(provider) => *provider,
            None => config.provider,
        });

        args.language = Some(match &args.language {
            Some(language) => *language,
            None => config.subs_language,
        });

        args.debug = if !args.debug {
            config.debug
        } else {
            args.debug
        };

        args
    }
}
