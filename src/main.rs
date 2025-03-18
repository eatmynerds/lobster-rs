use anyhow::anyhow;
use clap::{Parser, ValueEnum};
use futures::future::{BoxFuture, FutureExt};
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, error, info, warn, LevelFilter};
use regex::Regex;
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
use utils::history::{save_history, save_progress};
use utils::image_preview::remove_desktop_and_tmp;
use utils::presence::discord_presence;
use utils::SpawnError;

mod cli;
use cli::{run, subtitles_prompt};
mod flixhq;
use flixhq::flixhq::{FlixHQ, FlixHQEpisode, FlixHQSourceType, FlixHQSubtitles};
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
        iina::{Iina, IinaArgs, IinaPlay},
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
    Iina,
    SyncPlay,
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
pub enum Quality {
    #[clap(name = "360")]
    Q360 = 360,
    #[clap(name = "720")]
    Q720 = 720,
    #[clap(name = "1080")]
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
            0..=600 => Quality::Q360,
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
    Turkish,
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
            Languages::Turkish => write!(f, "Turkish"),
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
}

fn fzf_launcher<'a>(args: &'a mut FzfArgs) -> anyhow::Result<String> {
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
        return Err(anyhow!("No selection made. Exiting..."));
    }

    Ok(output)
}

fn rofi_launcher<'a>(args: &'a mut RofiArgs) -> anyhow::Result<String> {
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
        return Err(anyhow!("No selection made. Exiting..."));
    }

    Ok(output)
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
                chafa -f sixels -s 80x40 "/tmp/images/${selected}.jpg"
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
        match rofi_launcher(rofi_args) {
            Ok(output) => output,
            Err(_) => {
                if !image_preview_files.is_empty() {
                    for (_, _, media_id) in image_preview_files {
                        remove_desktop_and_tmp(media_id.to_string())
                            .expect("Failed to remove old .desktop files & tmp images");
                    }
                }

                std::process::exit(1)
            }
        }
    } else {
        debug!("Using fzf launcher.");
        match fzf_launcher(fzf_args) {
            Ok(output) => output,
            Err(_) => {
                if !image_preview_files.is_empty() {
                    for (_, _, media_id) in image_preview_files {
                        remove_desktop_and_tmp(media_id.to_string())
                            .expect("Failed to remove old .desktop files & tmp images");
                    }
                }

                std::process::exit(1)
            }
        }
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

async fn url_quality(url: String, quality: Option<Quality>) -> anyhow::Result<String> {
    let input = CLIENT.get(url).send().await?.text().await?;

    let url_re = Regex::new(r"https://[^\s]+m3u8").unwrap();
    let res_re = Regex::new(r"RESOLUTION=(\d+)x(\d+)").unwrap();

    let mut resolutions = Vec::new();
    for cap in res_re.captures_iter(&input) {
        resolutions.push(cap[2].to_string()); // Collect only height (e.g., "1080", "720", "360")
    }

    let url = if let Some(chosen_quality) = quality {
        url_re
            .captures_iter(&input)
            .zip(res_re.captures_iter(&input))
            .find_map(|(url_captures, res_captures)| {
                let resolution = &res_captures[2];
                let url = &url_captures[0];

                if resolution == chosen_quality.to_string() {
                    Some(url.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                info!("Quality {} not found, falling back to auto", chosen_quality);
                input
                    .lines()
                    .find(|line| line.starts_with("https://"))
                    .unwrap_or("")
                    .to_string()
            })
    } else {
        let mut urls_and_resolutions: Vec<(u32, String)> = url_re
            .captures_iter(&input)
            .zip(res_re.captures_iter(&input))
            .filter_map(|(url_captures, res_captures)| {
                let resolution: u32 = res_captures[2].parse().ok()?;
                let url = url_captures[0].to_string();
                Some((resolution, url))
            })
            .collect();

        urls_and_resolutions.sort_by_key(|&(resolution, _)| std::cmp::Reverse(resolution));

        let (_, url) = urls_and_resolutions
            .first()
            .expect("Failed to find best url quality!");

        url.to_string()
    };

    Ok(url)
}

async fn player_run_choice(
    media_info: (String, String, String, String),
    episode_info: Option<(usize, usize, Vec<Vec<FlixHQEpisode>>)>,
    config: Arc<Config>,
    settings: Arc<Args>,
    player: Player,
    download_dir: Option<String>,
    player_url: String,
    subtitles: Vec<String>,
    subtitle_language: Option<Languages>,
) -> anyhow::Result<()> {
    let process_stdin = if media_info.1.starts_with("tv/") {
        Some("Next Episode\nPrevious Episode\nReplay\nExit\nSearch".to_string())
    } else {
        Some("Replay\nExit\nSearch".to_string())
    };

    let run_choice = launcher(
        &vec![],
        settings.rofi,
        &mut RofiArgs {
            mesg: Some("Select: ".to_string()),
            process_stdin: process_stdin.clone(),
            dmenu: true,
            case_sensitive: true,
            ..Default::default()
        },
        &mut FzfArgs {
            prompt: Some("Select: ".to_string()),
            process_stdin,
            reverse: true,
            ..Default::default()
        },
    )
    .await;

    match run_choice.as_str() {
        "Next Episode" => {
            handle_servers(
                config.clone(),
                settings.clone(),
                Some(true),
                (
                    media_info.0.as_str(),
                    media_info.1.as_str(),
                    media_info.2.as_str(),
                    media_info.3.as_str(),
                ),
                episode_info,
            )
            .await?;
        }
        "Previous Episode" => {
            handle_servers(
                config.clone(),
                settings.clone(),
                Some(false),
                (
                    media_info.0.as_str(),
                    media_info.1.as_str(),
                    media_info.2.as_str(),
                    media_info.3.as_str(),
                ),
                episode_info,
            )
            .await?;
        }
        "Search" => {
            run(Arc::new(Args::default()), Arc::clone(&config)).await?;
        }
        "Replay" => {
            handle_stream(
                settings.clone(),
                config.clone(),
                player,
                download_dir,
                player_url,
                media_info,
                episode_info,
                subtitles,
                subtitle_language,
            )
            .await?;
        }
        "Exit" => {
            std::process::exit(0);
        }
        _ => {
            unreachable!("You shouldn't be here...")
        }
    }

    Ok(())
}

fn handle_stream(
    settings: Arc<Args>,
    config: Arc<Config>,
    player: Player,
    download_dir: Option<String>,
    url: String,
    media_info: (String, String, String, String),
    episode_info: Option<(usize, usize, Vec<Vec<FlixHQEpisode>>)>,
    subtitles: Vec<String>,
    subtitle_language: Option<Languages>,
) -> BoxFuture<'static, anyhow::Result<()>> {
    let subtitles_choice = subtitles_prompt();
    let player_url = url.clone();

    let subtitles_for_player = if subtitles_choice {
        if subtitles.len() > 0 {
            Some(subtitles.clone())
        } else {
            info!("No subtitles available!");
            None
        }
    } else {
        info!("Continuing without subtitles");
        None
    };

    let subtitle_language = if subtitles_choice {
        subtitle_language
    } else {
        None
    };

    async move {
        match player {
            Player::Iina => {
                let iina = Iina::new();

                iina.play(IinaArgs {
                    url,
                    no_stdin: true,
                    keep_running: true,
                    mpv_sub_files: subtitles_for_player,
                    mpv_force_media_title: Some(media_info.2.clone()),
                    ..Default::default()
                })?;
            }
            Player::Vlc => {
                if let Some(download_dir) = download_dir {
                    download(
                        download_dir,
                        media_info.2,
                        url,
                        subtitles_for_player,
                        subtitle_language,
                    )
                    .await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let url = url_quality(url, settings.quality).await?;

                let vlc = Vlc::new();

                vlc.play(VlcArgs {
                    url,
                    input_slave: subtitles_for_player,
                    meta_title: Some(media_info.2.clone()),
                    ..Default::default()
                })?;

                player_run_choice(
                    media_info,
                    episode_info,
                    config,
                    settings,
                    player,
                    download_dir,
                    player_url,
                    subtitles,
                    subtitle_language,
                )
                .await?;
            }
            Player::Mpv => {
                if let Some(download_dir) = download_dir {
                    download(
                        download_dir,
                        media_info.2,
                        url,
                        subtitles_for_player.clone(),
                        subtitle_language,
                    )
                    .await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let watchlater_dir = std::path::PathBuf::new().join("/tmp/lobster-rs/watchlater");

                if watchlater_dir.exists() {
                    std::fs::remove_dir_all(&watchlater_dir)
                        .expect("Failed to remove watchlater directory!");
                }

                std::fs::create_dir_all(&watchlater_dir)
                    .expect("Failed to create watchlater directory!");

                let url = url_quality(url, settings.quality).await?;

                let mpv = Mpv::new();

                let mut child = mpv.play(MpvArgs {
                    url: url.clone(),
                    sub_files: subtitles_for_player.clone(),
                    force_media_title: Some(media_info.2.clone()),
                    watch_later_dir: Some(String::from("/tmp/lobster-rs/watchlater")),
                    write_filename_in_watch_later_config: true,
                    save_position_on_quit: true,
                    ..Default::default()
                })?;

                if config.history {
                    let (position, progress) = save_progress(url).await?;

                    save_history(media_info.clone(), episode_info.clone(), position, progress)
                        .await?;
                }

                if settings.rpc {
                    let season_and_episode_num = episode_info.as_ref().map(|(a, b, _)| (*a, *b));

                    discord_presence(
                        &media_info.2.clone(),
                        season_and_episode_num,
                        child,
                        &media_info.3,
                    )
                    .await?;
                } else {
                    child.wait()?;
                }

                player_run_choice(
                    media_info,
                    episode_info,
                    config,
                    settings,
                    player,
                    download_dir,
                    player_url,
                    subtitles,
                    subtitle_language,
                )
                .await?;
            }
            Player::SyncPlay => {
                let url = url_quality(url, settings.quality).await?;

                Command::new("nohup")
                    .args([
                        r#""syncplay""#,
                        &url,
                        "--",
                        &format!("--force-media-title={}", media_info.2),
                    ])
                    .spawn()
                    .map_err(|e| {
                        error!("Failed to start Syncplay: {}", e);
                        SpawnError::IOError(e)
                    })?;
            }
        }

        Ok(())
    }
    .boxed()
}

pub async fn handle_servers(
    config: Arc<Config>,
    settings: Arc<Args>,
    next_episode: Option<bool>,
    media_info: (&str, &str, &str, &str),
    episode_info: Option<(usize, usize, Vec<Vec<FlixHQEpisode>>)>,
) -> anyhow::Result<()> {
    debug!(
        "Fetching servers for episode_id: {}, media_id: {}",
        media_info.0, media_info.1
    );

    let (episode_id, server_results) = if let Some(next_episode) = next_episode {
        let episode_info = episode_info.clone().expect("Failed to get episode info");
        let mut episode_number = episode_info.1; // Current episode
        let mut season_number = episode_info.0; // Current season

        let total_seasons = episode_info.2.len();

        if next_episode {
            let total_episodes = episode_info.2[season_number - 1].len();

            if episode_number < total_episodes {
                // Move to next episode
                episode_number += 1;
            } else if season_number < total_seasons {
                // Move to the first episode of the next season
                season_number += 1;
                episode_number = 1;
            } else {
                // No next episode or season available, staying at the last episode
                eprintln!("No next episode or season available.");
                std::process::exit(1);
            }
        } else {
            // Move to the previous episode
            if episode_number > 1 {
                episode_number -= 1;
            } else if season_number > 1 {
                // Move to the last episode of the previous season
                season_number -= 1;
                episode_number = episode_info.2[season_number - 1].len();
            }
        }

        let episode_id = episode_info.2[season_number - 1][episode_number].id.clone();

        (
            episode_id.clone(),
            FlixHQ
                .servers(&episode_id, media_info.1)
                .await
                .map_err(|_| anyhow::anyhow!("Timeout while fetching servers"))?,
        )
    } else {
        (
            media_info.0.to_string(),
            FlixHQ
                .servers(media_info.0, media_info.1)
                .await
                .map_err(|_| anyhow::anyhow!("Timeout while fetching servers"))?,
        )
    };

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

    let sources = FlixHQ
        .sources(episode_id.as_str(), media_info.1, *server)
        .await
        .map_err(|_| anyhow::anyhow!("Timeout while fetching sources"))?;

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

            let mut player = match config.player.to_lowercase().as_str() {
                "vlc" => Player::Vlc,
                "mpv" => Player::Mpv,
                "syncplay" => Player::SyncPlay,
                "iina" => Player::Iina,
                _ => {
                    error!("Player not supported");
                    std::process::exit(1);
                }
            };

            if settings.syncplay {
                player = Player::SyncPlay;
            }

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
                (
                    media_info.0.to_string(),
                    media_info.1.to_string(),
                    media_info.2.to_string(),
                    media_info.3.to_string(),
                ),
                episode_info.map(|(a, b, c)| (a, b, c)),
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

    if args.edit {
        if cfg!(not(target_os = "windows")) {
            let editor = std::env::var("EDITOR").expect("EDITOR environment variable not set");
            std::process::Command::new(editor)
                .arg(
                    dirs::config_dir()
                        .expect("Failed to get config directory")
                        .join("lobster-rs/config.toml"),
                )
                .status()
                .expect("Failed to open config file with editor");

            info!("Done editing config file.");
        } else {
            info!("The `edit` flag is not supported on Windows.");
        }
    }

    let config = Arc::new(Config::load_config().expect("Failed to load config file"));

    let settings = Arc::new(Config::program_configuration(args, &config));

    run(settings, config).await?;

    Ok(())
}
