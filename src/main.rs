use clap::{Parser, ValueEnum};
use log::LevelFilter;
use log::error;
use log::info;
use log::warn;
use self_update::cargo_crate_version;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;
use strum::IntoEnumIterator;
use thiserror::Error;

mod cli;
mod providers;
use cli::{Languages, Player, Provider, Quality, cli::run};
mod flixhq;
mod utils;
use lazy_static::lazy_static;
use reqwest::Client;
use utils::config::Config;

lazy_static! {
    static ref CLIENT: Client = Client::new();
}

#[derive(ValueEnum, Debug, Clone, Serialize, Deserialize)]
#[clap(rename_all = "kebab-case")]
pub enum MediaType {
    Tv,
    Movie,
}

impl Display for MediaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MediaType::Tv => write!(f, "tv"),
            MediaType::Movie => write!(f, "movie"),
        }
    }
}

#[derive(Parser, Debug, Clone, Default)]
#[clap(author, version, about = "A media streaming CLI tool", long_about = None)]
pub struct Args {
    /// The search query or title to look for
    #[clap(value_parser)]
    pub query: Option<String>,

    /// Deletes the history file
    #[clap(long)]
    pub clear_history: bool,

    /// Continue watching from current history
    #[clap(short = 'R', long,aliases = ["continue"],visible_short_alias = 'c', visible_alias = "continue" )]
    pub resume: bool,

    /// Downloads movie or episode that is selected (defaults to current directory)
    #[clap(short, long)]
    pub download: Option<Option<String>>,

    /// Enables discord rich presence (beta feature, works fine on Linux)
    #[clap(short, long)]
    pub rpc: bool,

    /// Edit config file using an editor defined with lobster_editor in the config ($EDITOR by default)
    #[clap(short, long)]
    pub edit: bool,

    /// Shows image previews during media selection
    #[clap(short, long)]
    pub image_preview: bool,

    /// Outputs JSON containing video links, subtitle links, etc.
    #[clap(short, long)]
    pub json: bool,

    /// Specify the subtitle language
    #[clap(short, long, value_enum)]
    pub language: Option<Languages>,

    /// Use rofi instead of fzf
    #[clap(long)]
    pub rofi: bool,

    /// Specify the provider to watch from
    #[clap(short, long, value_enum)]
    pub provider: Option<Provider>,

    /// Specify the video quality (defaults to the highest possible quality)
    #[clap(short, long, value_enum)]
    pub quality: Option<Quality>,

    /// Lets you select from the most recent movies or TV shows
    #[clap(long, value_enum)]
    pub recent: Option<MediaType>,

    /// Use Syncplay to watch with friends
    #[clap(short, long)]
    pub syncplay: bool,

    /// Lets you select from the most popular movies or TV shows
    #[clap(short, long, value_enum)]
    pub trending: Option<MediaType>,

    /// Update the script
    #[clap(short, long)]
    pub update: bool,

    /// Enable debug mode (prints debug info to stdout and saves it to $TEMPDIR/lobster.log)
    #[clap(long)]
    pub debug: bool,

    /// Disable subtitles
    #[clap(short, long)]
    pub no_subs: bool,
}


#[derive(Debug, Error)]
enum CliError {
    #[error("No compatible video players were found, please install MPV")]
    NoPlayersInstalled,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Dependencies {
    player: Player,
    fzf: bool,
    rofi: bool,
    ffmpeg: bool,
    chafa: bool,
}

impl Dependencies {
    const DEPENDENCY_LIST: &'static [&str] = if cfg!(target_os = "windows") {
        &["chafa", "ffmpeg", "fzf"]
    } else {
        &["fzf", "rofi", "ffmpeg", "chafa"]
    };
    pub fn is_command_available<C: AsRef<str>>(command: C) -> bool {
        const DELIMITER: char = if cfg!(windows) { ';' } else { ':' };
        let system_path = match env::var("PATH") {
            Ok(list) => list,
            Err(e) => {
                error!("System path was not accessible? no dependencies can be fetched!: {e}");
                return false;
            }
        };
        let is_avalible = env::split_paths(&system_path).into_iter().any(|mut dir| {
            dir.push(command.as_ref());
            if cfg!(windows) {
                let mut path_with_ext = dir.clone();
                path_with_ext.set_extension("exe");
                if path_with_ext.is_file() {
                    return true;
                }
            }
            dir.is_file()
        });
        is_avalible
    }
    fn get_avalible() -> Self {
        let mut dependencies = Dependencies::default();
        let supported_players: Vec<String> = Player::iter().map(|p| p.to_string()).collect();
        let mut avalible_players: Vec<Player> = vec![];
        for player in supported_players {
            
        }
        // NEEDED: fzf
        // WARN: chafa (image preview)
        // WARN: ffmpeg (downloading)
        // WARN: rofi (fzf alternative GUI)

        dependencies.fzf = match Self::is_command_available("fzf") {
            true => true,
            false => {
                error!("fzf is not avalible!");
                std::process::exit(1);
            }
        };
        dependencies.ffmpeg = match Self::is_command_available("ffmpeg") {
            true => true,
            false => {
                error!("ffmpeg is not avalible! downloading will not work");
                false
            }
        };
        #[cfg(windows)]
        {
            dependencies.chafa = match Self::is_command_available("chafa") {
                true => true,
                false => {
                    warn!("chafa is not avalible! image previews will not work");
                    false
                }
            };
            dependencies.rofi = match Self::is_command_available("rofi") {
                true => true,
                false => {
                    warn!("rofi is not avalible! image previews will not work");
                    false
                }
            };
        }
        // check_command!(chafa,"chafa","blahblahblah no chafa looser");
        // check_command!(rofi,"rofi","blahblahblah no chafa looser");
        // check_command!(fzf,"fzf","blahblahblah no chafa looser");
        // check_command!(ffmpeg,"ffmpeg","blahblahblah no chafa looser");

        dependencies
    }
}

fn update() -> anyhow::Result<()> {
    let target = self_update::get_target();

    let target_arch = match target {
        "x86_64-unknown-linux-gnu" => "x86_64-unknown-linux-gnu_lobster-rs",
        "aarch64-unknown-linux-gnu" => "aarch64-unknown-linux-gnu_lobster-rs",
        "x86_64-apple-darwin" => "x86_64-apple-darwin_lobster-rs",
        "aarch64-apple-darwin" => "aarch64-apple-darwin_lobster-rs",
        "x86_64-pc-windows-msvc" => "x86_64-pc-windows-msvc_lobster-rs.exe",
        "aarch64-pc-windows-msvc" => "aarch64-pc-windows-msvc_lobster-rs.exe",
        _ => return Err(anyhow::anyhow!("Unsupported target: {}", target)),
    };

    let status = self_update::backends::github::Update::configure()
        .repo_owner("eatmynerds")
        .repo_name("lobster-rs")
        .bin_name(target_arch)
        .target("lobster-rs")
        .current_version(cargo_crate_version!())
        .show_download_progress(true)
        .build()?
        .update()?;

    println!("Update status: Updated to version `{}`!", status.version());

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_level = if args.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    rich_logger::init(log_level).expect("Failed to initalize logger: {e}");

    let _deps = Dependencies::get_avalible();

    if args.update {
        let update_result = tokio::task::spawn_blocking(move || update()).await.unwrap();

        match update_result {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(e) => {
                error!("Failed to update: {}", e);
                std::process::exit(1);
            }
        }
    };

    if args.edit {
        if cfg!(not(target_os = "windows")) {
            let editor = std::env::var("EDITOR")
                .map_err(|_| {
                    error!("EDITOR environment variable not set!");
                    std::process::exit(1);
                })
                .unwrap();
            std::process::Command::new(editor)
                .arg(
                    dirs::config_dir()
                        .expect("Failed to get config directory")
                        .join("lobster-rs/config.toml"),
                )
                .status()
                .expect("Failed to open config file with editor");

            info!("Done editing config file.");
            std::process::exit(0);
        } else {
            error!("The `edit` flag is not supported on Windows.");
            std::process::exit(1);
        }
    }

    let config = Arc::new(Config::load_config().expect("Failed to load config file"));

    let settings = Arc::new(Config::program_configuration(args, &config));

    run(settings, config).await?;

    Ok(())
}
