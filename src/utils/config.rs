use crate::{Args, Languages, Provider};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
use tracing::{debug, warn};

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
        debug!("Creating a new default configuration.");
        let download_dir = std::env::current_dir()
            .expect("Failed to get current directory")
            .to_str()
            .expect("Failed to convert path to str")
            .to_string();

        let histfile = format!(
            "{}/lobster/lobster_history.txt",
            dirs::config_dir()
                .expect("Failed to get configuration directory")
                .display()
        );

        Self {
            player: String::from("mpv"),
            download: download_dir,
            provider: Provider::Vidcloud,
            history: false,
            subs_language: Languages::English,
            histfile,
            use_external_menu: false,
            image_preview: false,
            debug: false,
        }
    }

    pub fn load_config() -> anyhow::Result<Self> {
        debug!("Loading configuration...");
        let config_dir = dirs::config_dir().context("Failed to retrieve the config directory")?;

        let config_path = format!("{}/lobster_rs/config.toml", config_dir.display());
        debug!("Looking for config file at path: {}", config_path);

        let config = Config::load_from_file(Path::new(&config_path))?;
        debug!("Configuration loaded successfully.");
        Ok(config)
    }

    pub fn load_from_file(file_path: &Path) -> anyhow::Result<Self> {
        if !file_path.exists() {
            warn!(
                "Config file not found at {:?}. Creating a default configuration.",
                file_path
            );

            let default_config = Config::new();
            let content = toml::to_string(&default_config)
                .with_context(|| "Failed to serialize the default configuration")?;

            if let Some(parent) = file_path.parent() {
                debug!("Creating config directory: {:?}", parent);
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
            }

            let mut file = File::create(&file_path)
                .with_context(|| format!("Failed to create config file: {:?}", file_path))?;

            debug!("Writing default configuration to file.");
            file.write_all(content.as_bytes())
                .with_context(|| format!("Failed to write to config file: {:?}", file_path))?;

            debug!("Default configuration created successfully.");
            return Ok(default_config);
        }

        debug!("Reading config file from {:?}", file_path);
        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read config file: {:?}", file_path))?;

        debug!("Parsing config file content.");
        toml::from_str(&content).context("Failed to parse config.toml")
    }

    pub fn program_configuration<'a>(args: &'a mut Args, config: &Self) -> &'a mut Args {
        debug!("Applying configuration to program arguments.");

        if cfg!(target_os = "linux") {
            args.rofi = if !args.rofi {
                debug!("Setting `rofi` to {}", config.use_external_menu);
                config.use_external_menu
            } else {
                args.rofi
            };
        } else {
            debug!("Disabling `rofi` as it is not supported on this OS.");
            args.rofi = false;
        }

        args.image_preview = if !args.image_preview {
            debug!("Setting `image_preview` to {}", config.image_preview);
            config.image_preview
        } else {
            args.image_preview
        };

        args.download = args.download.as_ref().map(|download| {
            if download.is_some() {
                debug!("Using provided download directory: {:?}", download);
            } else {
                warn!("Provided download directory is empty. Using default download directory.");
                debug!("Using default download directory: {:?}", config.download);
            }
            Some(download.clone().unwrap_or_else(|| config.download.clone()))
        });

        args.provider = Some(match &args.provider {
            Some(provider) => {
                debug!("Using provided provider: {:?}", provider);
                *provider
            }
            None => {
                debug!("Using default provider: {:?}", config.provider);
                config.provider
            }
        });

        args.language = Some(match &args.language {
            Some(language) => {
                debug!("Using provided language: {:?}", language);
                *language
            }
            None => {
                debug!("Using default language: {:?}", config.subs_language);
                config.subs_language
            }
        });

        args.debug = if !args.debug {
            debug!("Setting `debug` to {}", config.debug);
            config.debug
        } else {
            args.debug
        };

        args
    }
}
