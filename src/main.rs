use clap::{Parser, ValueEnum};
use futures::future::{BoxFuture, FutureExt};
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, error, info, warn, LevelFilter};
use reqwest::Client;
use self_update::cargo_crate_version;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Debug, Display, Formatter},
    num::ParseIntError,
    process::Command,
    str::FromStr,
    sync::Arc,
};
use tokio::signal;
use tokio::time::Duration;

mod cli;
use cli::{run, subtitles_prompt};
mod flixhq;
use flixhq::flixhq::{FlixHQ, FlixHQSourceType, FlixHQSubtitles};
mod providers;
mod utils;
use utils::{
    config::Config,
    ffmpeg::{Ffmpeg, FfmpegArgs, FfmpegSpawn},
    fzf::{Fzf, FzfArgs, FzfSpawn},
    image_preview::{generate_desktop, image_preview},
    players::{
        mpv::{Mpv, MpvArgs, MpvPlay},
        vlc::{Vlc, VlcArgs, VlcPlay},
    },
    rofi::{Rofi, RofiArgs, RofiSpawn},
};

pub static BASE_URL: &'static str = "https://flixhq.to";

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

#[derive(Debug)]
pub enum Player {
    Vlc,
    Mpv,
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, Copy, PartialEq)]
#[clap(rename_all = "PascalCase")]
pub enum Provider {
    Vidcloud,
    Upcloud,
}

impl Display for Provider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Vidcloud => write!(f, "Vidcloud"),
            Provider::Upcloud => write!(f, "Upcloud"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
#[clap(rename_all = "kebab-case")]
pub enum Quality {
    Q240 = 240,
    Q360 = 360,
    Q480 = 480,
    Q720 = 720,
    Q1080 = 1080,
}

#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("Failed to parse quality from string: {0}")]
    QualityParseError(#[from] ParseIntError),
}

impl FromStr for Quality {
    type Err = StreamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let quality = s.parse::<u32>()?;
        Ok(match quality {
            0..=300 => Quality::Q240,
            301..=420 => Quality::Q360,
            421..=600 => Quality::Q480,
            601..=840 => Quality::Q720,
            841..=1200 => Quality::Q1080,
            _ => Quality::Q1080,
        })
    }
}

impl Quality {
    fn to_u32(self) -> u32 {
        self as u32
    }
}

impl Display for Quality {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_u32())
    }
}

#[derive(ValueEnum, Debug, Clone, Serialize, Deserialize, Copy)]
#[clap(rename_all = "PascalCase")]
pub enum Languages {
    Arabic,
    Danish,
    Dutch,
    English,
    Finnish,
    German,
    Italian,
    Russian,
    Spanish,
}

impl Display for Languages {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Languages::Arabic => write!(f, "Arabic"),
            Languages::Danish => write!(f, "Danish"),
            Languages::Dutch => write!(f, "Dutch"),
            Languages::English => write!(f, "English"),
            Languages::Finnish => write!(f, "Finnish"),
            Languages::German => write!(f, "German"),
            Languages::Italian => write!(f, "Italian"),
            Languages::Russian => write!(f, "Russian"),
            Languages::Spanish => write!(f, "Spanish"),
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about = "A media streaming CLI tool", long_about = None)]
pub struct Args {
    /// The search query or title to look for
    #[clap(value_parser)]
    pub query: Option<String>,

    /// Continue watching from current history
    #[clap(short, long)]
    pub r#continue: bool,

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

    /// Specify the video quality
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
}

fn fzf_launcher<'a>(args: &'a mut FzfArgs) -> String {
    debug!("Launching fzf with arguments: {:?}", args);

    let mut fzf = Fzf::new();

    let output = fzf
        .spawn(args)
        .map(|output| {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            debug!("fzf completed with result: {}", result);
            result
        })
        .unwrap_or_else(|e| {
            error!("Failed to launch fzf: {}", e.to_string());
            std::process::exit(1)
        });

    if output.is_empty() {
        error!("No selection made. Exiting...");
        std::process::exit(1)
    }

    output
}

fn rofi_launcher<'a>(args: &'a mut RofiArgs) -> String {
    debug!("Launching rofi with arguments: {:?}", args);

    let mut rofi = Rofi::new();

    let output = rofi
        .spawn(args)
        .map(|output| {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            debug!("rofi completed with result: {}", result);
            result
        })
        .unwrap_or_else(|e| {
            error!("Failed to launch rofi: {}", e.to_string());
            std::process::exit(1)
        });

    if output.is_empty() {
        error!("No selection made. Exiting...");
        std::process::exit(1)
    }

    output
}

async fn launcher(
    image_preview_files: &Vec<(String, String, String)>,
    rofi: bool,
    rofi_args: &mut RofiArgs,
    fzf_args: &mut FzfArgs,
) -> String {
    if image_preview_files.is_empty() {
        debug!("No image preview files provided.");
    } else {
        debug!(
            "Generating image previews for {} files.",
            image_preview_files.len()
        );
        let temp_images_dirs = image_preview(image_preview_files)
            .await
            .expect("Failed to generate image previews");

        if rofi {
            for (media_name, media_id, image_path) in temp_images_dirs {
                debug!(
                    "Generating desktop entry for: {} (ID: {})",
                    media_name, media_id
                );
                generate_desktop(media_name, media_id, image_path)
                    .expect("Failed to generate desktop entry for image preview");
            }

            rofi_args.show = Some("drun".to_string());
            rofi_args.drun_categories = Some("imagepreview".to_string());
            rofi_args.show_icons = true;
            rofi_args.dmenu = false;
        } else {
            match std::process::Command::new("chafa").arg("-v").output() {
                Ok(_) => {
                    debug!("Setting up fzf preview script.");

                    fzf_args.preview = Some(
                        r#"
                selected=$(echo {} | cut -f2 | sed 's/\//-/g')
                chafa -f sixel -s 80x40 "/tmp/images/${selected}.jpg"
                    "#
                        .to_string(),
                    );
                }
                Err(_) => {
                    warn!("Chafa isn't installed. Cannot preview images with fzf.");
                }
            }
        }
    }

    if rofi {
        debug!("Using rofi launcher.");
        rofi_launcher(rofi_args)
    } else {
        debug!("Using fzf launcher.");
        fzf_launcher(fzf_args)
    }
}

async fn download(
    download_dir: String,
    media_title: String,
    url: String,
    subtitles: Option<Vec<String>>,
    subtitle_language: Option<Languages>,
) -> anyhow::Result<()> {
    info!("{}", format!(r#"Starting download for "{}""#, media_title));

    let ffmpeg = Ffmpeg::new();

    ffmpeg.embed_video(FfmpegArgs {
        input_file: url,
        log_level: Some("error".to_string()),
        stats: true,
        output_file: format!("{}/{}.mkv", download_dir, media_title),
        subtitle_files: subtitles.as_ref(),
        subtitle_language: Some(subtitle_language.unwrap_or(Languages::English).to_string()),
        codec: Some("copy".to_string()),
    })?;

    Ok(())
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

async fn handle_ctrl_c() {
    signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
}

fn handle_stream(
    settings: Arc<Args>,
    config: Arc<Config>,
    player: Player,
    download_dir: Option<String>,
    url: String,
    media_title: String,
    subtitles: Vec<String>,
    subtitle_language: Option<Languages>,
) -> BoxFuture<'static, anyhow::Result<()>> {
    tokio::spawn(handle_ctrl_c());

    let subtitles_choice = subtitles_prompt();

    let (subtitles, subtitle_language) = if subtitles_choice {
        (Some(subtitles), subtitle_language)
    } else {
        (None, None)
    };

    async move {
        match player {
            Player::Vlc => {
                if let Some(download_dir) = download_dir {
                    download(download_dir, media_title, url, subtitles, subtitle_language).await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let vlc = Vlc::new();

                vlc.play(VlcArgs {
                    url,
                    input_slave: None,
                    meta_title: Some(media_title),
                    ..Default::default()
                })?;

                let run_choice = launcher(
                    &vec![],
                    settings.rofi,
                    &mut RofiArgs {
                        mesg: Some("Select: ".to_string()),
                        process_stdin: Some("Exit\nSearch".to_string()),
                        dmenu: true,
                        case_sensitive: true,
                        ..Default::default()
                    },
                    &mut FzfArgs {
                        prompt: Some("Select: ".to_string()),
                        process_stdin: Some("Exit\nSearch".to_string()),
                        reverse: true,
                        ..Default::default()
                    },
                )
                .await;

                match run_choice.as_str() {
                    "Search" => {
                        run(Arc::clone(&settings), Arc::clone(&config)).await?;
                    }
                    "Exit" => {
                        info!("Exiting...");
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            Player::Mpv => {
                if let Some(download_dir) = download_dir {
                    download(download_dir, media_title, url, subtitles, subtitle_language).await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let mpv = Mpv::new();

                mpv.play(MpvArgs {
                    url,
                    sub_files: subtitles,
                    force_media_title: Some(media_title),
                    msg_level: Some("error".to_string()),
                    ..Default::default()
                })?;

                let run_choice = launcher(
                    &vec![],
                    settings.rofi,
                    &mut RofiArgs {
                        mesg: Some("Select: ".to_string()),
                        process_stdin: Some("Exit\nSearch".to_string()),
                        dmenu: true,
                        case_sensitive: true,
                        ..Default::default()
                    },
                    &mut FzfArgs {
                        prompt: Some("Select: ".to_string()),
                        process_stdin: Some("Exit\nSearch".to_string()),
                        reverse: true,
                        ..Default::default()
                    },
                )
                .await;

                match run_choice.as_str() {
                    "Search" => {
                        run(Arc::clone(&settings), Arc::clone(&config)).await?;
                    }
                    "Exit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
    .boxed()
}

pub async fn handle_servers(
    config: Arc<Config>,
    settings: Arc<Args>,
    episode_id: &str,
    media_id: &str,
    media_title: &str,
) -> anyhow::Result<()> {
    debug!(
        "Fetching servers for episode_id: {}, media_id: {}",
        episode_id, media_id
    );

    let server_results = tokio::time::timeout(
        Duration::from_secs(10),
        FlixHQ.servers(episode_id, media_id),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Timeout while fetching servers"))??;

    if server_results.servers.is_empty() {
        return Err(anyhow::anyhow!("No servers found"));
    }

    let servers: Vec<Provider> = server_results
        .servers
        .into_iter()
        .filter_map(|server_result| match server_result.name.as_str() {
            "Vidcloud" => Some(Provider::Vidcloud),
            "Upcloud" => Some(Provider::Upcloud),
            _ => None,
        })
        .collect();

    let server_choice = settings.provider.unwrap_or(Provider::Vidcloud);

    let server = servers
        .iter()
        .find(|&&x| x == server_choice)
        .unwrap_or(&Provider::Vidcloud);

    debug!("Fetching sources for selected server: {:?}", server);

    let sources = tokio::time::timeout(
        Duration::from_secs(10),
        FlixHQ.sources(episode_id, media_id, *server),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Timeout while fetching sources"))??;

    debug!("Fetched sources: {:?}", sources);

    if settings.json {
        info!("{}", serde_json::to_value(&sources).unwrap());
    }

    match (sources.sources, sources.subtitles) {
        (
            FlixHQSourceType::VidCloud(vidcloud_sources),
            FlixHQSubtitles::VidCloud(vidcloud_subtitles),
        ) => {
            if vidcloud_sources.is_empty() {
                return Err(anyhow::anyhow!("No sources available from VidCloud"));
            }

            debug!("Found subtitles: {:?}", vidcloud_subtitles);

            let selected_subtitles: Vec<String> = futures::stream::iter(vidcloud_subtitles)
                .filter(|subtitle| {
                    let settings = Arc::clone(&settings);
                    let subtitle_label = subtitle.label.clone();
                    async move {
                        let language = settings.language.unwrap_or(Languages::English).to_string();
                        subtitle_label.contains(&language)
                    }
                })
                .map(|subtitle| subtitle.file.clone())
                .collect()
                .await;

            debug!("Selected subtitles: {:?}", selected_subtitles);

            let player = match config.player.to_lowercase().as_str() {
                "vlc" => Player::Vlc,
                "mpv" => Player::Mpv,
                _ => {
                    error!("Player not supported");
                    std::process::exit(1);
                }
            };

            debug!("Starting stream with player: {:?}", player);

            handle_stream(
                Arc::clone(&settings),
                Arc::clone(&config),
                player,
                settings
                    .download
                    .as_ref()
                    .and_then(|inner| inner.as_ref())
                    .cloned(),
                vidcloud_sources[0].file.to_string(),
                media_title.to_string(),
                selected_subtitles,
                Some(settings.language.unwrap_or(Languages::English)),
            )
            .await?;
        }
    }

    Ok(())
}

fn is_command_available(command: &str) -> bool {
    let version_arg = if command == "rofi" || command == "ffmpeg" {
        String::from("-version")
    } else {
        String::from("--version")
    };

    match Command::new(command).arg(version_arg).output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn check_dependencies() {
    let dependencies = if cfg!(target_os = "windows") {
        vec!["mpv", "chafa", "ffmpeg", "fzf"]
    } else {
        vec!["mpv", "fzf", "rofi", "ffmpeg", "chafa"]
    };

    for dep in dependencies {
        if !is_command_available(dep) {
            match dep {
                "chafa" => {
                    warn!(
                        "Chafa isn't installed. You won't be able to do image previews with fzf."
                    );
                    continue;
                }
                "rofi" => {
                    warn!("Rofi isn't installed. You won't be able to use rofi to search.");
                    continue;
                }
                "ffmpeg" => {
                    warn!("Ffmpeg isn't installed. You won't be able to download.");
                    continue;
                }
                _ => {
                    error!("{} is missing. Please install it.", dep);
                    std::process::exit(1);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_level = if args.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    rich_logger::init(log_level).unwrap();

    check_dependencies();

    if args.update {
        let update_result = tokio::task::spawn_blocking(move || update()).await?;

        match update_result {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(e) => {
                error!("Failed to update: {}", e);
                std::process::exit(1);
            }
        }
    }

    if cfg!(target_os = "linux") {
        if args.edit {
            let editor = std::env::var("EDITOR").expect("EDITOR environment variable not set");
            std::process::Command::new(editor)
                .arg(
                    dirs::config_dir()
                        .expect("Failed to get config directory")
                        .join("lobster_rs/config.toml"),
                )
                .status()
                .expect("Failed to open config file with editor");

            info!("Done editing config file.");
        }
    } else {
        info!("The `edit` flag is only supported on Linux.");
    }

    let config = Arc::new(Config::load_config().expect("Failed to load config file"));

    let settings = Arc::new(Config::program_configuration(args, &config));

    run(settings, config).await?;

    Ok(())
}
