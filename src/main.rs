use anyhow::anyhow;
use clap::{Parser, ValueEnum};
use lazy_static::lazy_static;
use reqwest::Client;
use self_update::cargo_crate_version;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};
use tracing::{error, info, Level};

mod cli;
use cli::get_input;
mod flixhq;
use flixhq::flixhq::{FlixHQ, FlixHQInfo, FlixHQSourceType, FlixHQSubtitles};
mod providers;
mod utils;
use utils::{
    config::Config,
    ffmpeg::{Ffmpeg, FfmpegArgs, FfmpegSpawn},
    fzf::{Fzf, FzfArgs, FzfSpawn},
    image_preview::{generate_desktop, image_preview, remove_desktop_and_tmp},
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
enum MediaType {
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

enum Player {
    Vlc,
    Mpv,
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, Copy, PartialEq)]
#[clap(rename_all = "PascalCase")]
enum Provider {
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
enum Quality {
    Q240 = 240,
    Q360 = 360,
    Q480 = 480,
    Q720 = 720,
    Q1080 = 1080,
}

#[derive(thiserror::Error, Debug)]
enum StreamError {
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
enum Languages {
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

#[derive(Parser, Debug)]
#[clap(author, version, about = "A media streaming CLI tool", long_about = None)]
struct Args {
    /// The search query or title to look for
    #[clap(value_parser)]
    query: Option<String>,

    /// Continue watching from current history
    #[clap(short, long)]
    r#continue: bool,

    /// Downloads movie or episode that is selected (defaults to current directory)
    #[clap(short, long)]
    download: Option<Option<String>>,

    /// Enables discord rich presence (beta feature, works fine on Linux)
    #[clap(short, long)]
    rpc: bool,

    /// Edit config file using an editor defined with lobster_editor in the config ($EDITOR by default)
    #[clap(short, long)]
    edit: bool,

    /// Shows image previews during media selection
    #[clap(short, long)]
    image_preview: bool,

    /// Outputs JSON containing video links, subtitle links, etc.
    #[clap(short, long)]
    json: bool,

    /// Specify the subtitle language
    #[clap(short, long)]
    language: Option<Languages>,

    /// Use rofi instead of fzf
    #[clap(long)]
    rofi: bool,

    /// Specify the provider to watch from
    #[clap(short, long, value_enum)]
    provider: Option<Provider>,

    /// Specify the video quality
    #[clap(short, long, value_enum)]
    quality: Option<Quality>,

    /// Lets you select from the most recent movies or TV shows
    #[clap(long, value_enum)]
    recent: Option<MediaType>,

    /// Use Syncplay to watch with friends
    #[clap(short, long)]
    syncplay: bool,

    /// Lets you select from the most popular movies and shows
    #[clap(short, long)]
    trending: bool,

    /// Update the script
    #[clap(short, long)]
    update: bool,

    /// Enable debug mode (prints debug info to stdout and saves it to $TEMPDIR/lobster.log)
    #[clap(long)]
    debug: bool,
}

fn fzf_launcher<'a>(args: &'a mut FzfArgs) -> String {
    info!("Launching fzf with arguments: {:?}", args);

    let mut fzf = Fzf::new();

    let output = fzf
        .spawn(args)
        .map(|output| {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            info!("fzf completed with result: {}", result);
            result
        })
        .unwrap_or_else(|e| {
            error!("Failed to launch fzf: {}", e.to_string());
            std::process::exit(1)
        });

    if output.is_empty() {
        error!("No selection made.");
        std::process::exit(1)
    }

    output
}

fn rofi_launcher<'a>(args: &'a mut RofiArgs) -> String {
    info!("Launching rofi with arguments: {:?}", args);

    let mut rofi = Rofi::new();

    let output = rofi
        .spawn(args)
        .map(|output| {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            info!("rofi completed with result: {}", result);
            result
        })
        .unwrap_or_else(|e| {
            error!("Failed to launch rofi: {}", e.to_string());
            std::process::exit(1)
        });

    if output.is_empty() {
        error!("No selection made.");
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
    info!("Starting launcher with rofi: {}", rofi);

    if image_preview_files.is_empty() {
        info!("No image preview files provided.");
    } else {
        info!(
            "Generating image previews for {} files.",
            image_preview_files.len()
        );
        let temp_images_dirs = image_preview(image_preview_files)
            .await
            .expect("Failed to generate image previews");

        if rofi {
            for (media_name, media_id, image_path) in temp_images_dirs {
                info!(
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
            info!("Setting up fzf preview script.");

            fzf_args.preview = Some(
                r#"
            selected=$(echo {} | cut -f2 | sed 's/\//-/g')
            chafa -f sixel -s 80x40 "/tmp/images/${selected}.jpg"
                "#
                .to_string(),
            );
        }
    }

    if rofi {
        info!("Using rofi launcher.");
        rofi_launcher(rofi_args)
    } else {
        info!("Using fzf launcher.");
        println!("{:#?}", fzf_args);
        fzf_launcher(fzf_args)
    }
}

fn download(
    download_dir: String,
    media_title: String,
    url: String,
    _subtitles: Vec<String>,
    _subtitle_language: Option<Languages>,
) {
    info!(
        "Starting download for media: {} from URL: {}",
        media_title, url
    );

    let mut ffmpeg = Ffmpeg::new();

    let _ = ffmpeg
        .embed_video(&mut FfmpegArgs {
            input_file: url,
            log_level: Some("error".to_string()),
            stats: true,
            output_file: format!("{}/{}.mkv", download_dir, media_title),
            subtitle_files: None,
            subtitle_language: None,
            codec: Some("copy".to_string()),
        })
        .unwrap_or_else(|e| {
            error!("Failed to spawn ffmpeg: {}", e);
            std::process::exit(1)
        });
}

fn update() -> anyhow::Result<()> {
    let current_os = std::env::consts::OS;

    let binary_name = match current_os {
        "windows" => "lobster-rs-x86_64-windows.exe",
        "linux" => "lobster-rs-x86_64-unknown-linux-gnu",
        _ => {
            error!("Cannot update: current OS not supported!");
            std::process::exit(1)
        }
    };

    let status = self_update::backends::github::Update::configure()
        .repo_owner("eatmynerds")
        .repo_name("lobster-rs")
        .bin_name(binary_name)
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    info!("Update status: `{}`!", status.version());

    Ok(())
}

async fn handle_stream(
    player: Player,
    download_dir: Option<String>,
    url: String,
    media_title: String,
    subtitles: Vec<String>,
    subtitle_language: Option<Languages>,
) -> anyhow::Result<()> {
    match player {
        Player::Vlc => {
            if let Some(download_dir) = download_dir {
                download(download_dir, media_title, url, subtitles, subtitle_language);

                return Ok(());
            }

            let vlc = Vlc::new();

            let mut child = vlc
                .play(VlcArgs {
                    url,
                    input_slave: Some(subtitles),
                    meta_title: Some(media_title),
                    ..Default::default()
                })
                .unwrap();

            child
                .wait()
                .expect("Failed to spawn child process for vlc.");
        }
        Player::Mpv => {
            let mpv = Mpv::new();

            let mut child = mpv.play(MpvArgs {
                url,
                sub_files: Some(subtitles),
                force_media_title: Some(media_title),
                ..Default::default()
            })?;

            child
                .wait()
                .expect("Failed to spawn child process for mpv.");
        }
    }

    Ok(())
}

async fn handle_servers(
    config: Config,
    settings: &mut Args,
    episode_id: &str,
    media_id: &str,
    media_title: &str,
) -> anyhow::Result<()> {
    let server_results = FlixHQ.servers(episode_id, media_id).await?;

    let mut servers: Vec<Provider> = vec![];

    for server_result in server_results.servers {
        let provider = match server_result.name.as_str() {
            "Vidcloud" => Provider::Vidcloud,
            "Upcloud" => Provider::Upcloud,
            _ => continue,
        };
        servers.push(provider);
    }

    let server_choice = settings.provider.unwrap_or(Provider::Vidcloud);

    let server = servers
        .iter()
        .find(|&&x| x == server_choice)
        .unwrap_or(&Provider::Vidcloud);

    let sources = FlixHQ.sources(episode_id, media_id, *server).await?;

    match (sources.sources, sources.subtitles) {
        (
            FlixHQSourceType::VidCloud(vidcloud_sources),
            FlixHQSubtitles::VidCloud(vidcloud_subtitles),
        ) => {
            let mut selected_subtitles: Vec<String> = vec![];

            for subtitle in &vidcloud_subtitles {
                if subtitle
                    .label
                    .contains(&settings.language.unwrap_or(Languages::English).to_string())
                {
                    selected_subtitles.push(subtitle.file.to_string());
                }
            }

            match config.player.as_str() {
                "vlc" => {
                    handle_stream(
                        Player::Vlc,
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
                "mpv" => {
                    handle_stream(
                        Player::Mpv,
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
                _ => {
                    error!("Player not supported");
                    std::process::exit(1)
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .with_thread_names(true)
        .with_env_filter("lobster_rs=debug")
        .with_env_filter("none")
        .pretty()
        .init();

    let mut args = Args::parse();

    if args.update {
        let update_result = tokio::task::spawn_blocking(move || update()).await?;

        match update_result {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to update: {}", e);
                std::process::exit(1)
            }
        }
    }

    let config = Config::load_config().expect("Failed to load config file");

    let settings = Config::program_configuration(&mut args, &config);

    let query = match &settings.query {
        Some(query) => query.to_string(),
        None => get_input(settings.rofi)?,
    };

    let results = FlixHQ.search(&query).await?;

    if results.len() == 0 {
        return Err(anyhow!("No results found"));
    }

    let mut search_results: Vec<String> = vec![];
    let mut image_preview_files: Vec<(String, String, String)> = vec![];

    for result in results {
        match result {
            FlixHQInfo::Movie(movie) => {
                if settings.image_preview {
                    image_preview_files.push((
                        movie.title.to_string(),
                        movie.image.to_string(),
                        movie.id.to_string(),
                    ));
                }

                let movie_duration = movie.duration.replace("m", "").parse::<u32>()?;
                let formatted_duration = if movie_duration >= 60 {
                    let hours = movie_duration / 60;
                    let minutes = movie_duration % 60;
                    format!("{}h{}min", hours, minutes)
                } else {
                    format!("{}m", movie_duration)
                };

                search_results.push(format!(
                    "{}\t{}\t{}\t{} [{}] [{}]",
                    movie.image,
                    movie.id,
                    movie.media_type,
                    movie.title,
                    movie.year,
                    formatted_duration
                ));
            }
            FlixHQInfo::Tv(tv) => {
                if settings.image_preview {
                    image_preview_files.push((
                        tv.title.to_string(),
                        tv.image.to_string(),
                        tv.id.to_string(),
                    ));
                }

                search_results.push(format!(
                    "{}\t{}\t{}\t{} [SZNS {}] [EPS {}]",
                    tv.image, tv.id, tv.media_type, tv.title, tv.seasons.total_seasons, tv.episodes
                ))
            }
        }
    }

    let mut media_choice = launcher(
        &image_preview_files,
        settings.rofi,
        &mut RofiArgs {
            process_stdin: Some(search_results.join("\n")),
            mesg: Some("Choose a movie or TV show".to_string()),
            dmenu: true,
            case_sensitive: true,
            entry_prompt: Some("".to_string()),
            display_columns: Some(4),
            ..Default::default()
        },
        &mut FzfArgs {
            process_stdin: Some(search_results.join("\n")),
            reverse: true,
            with_nth: Some("4,5,6,7".to_string()),
            delimiter: Some("\t".to_string()),
            header: Some("Choose a movie or TV show".to_string()),
            ..Default::default()
        },
    )
    .await;

    if settings.image_preview {
        for (_, _, media_id) in &image_preview_files {
            remove_desktop_and_tmp(media_id.to_string())
                .expect("Failed to remove old .desktop files & tmp images");
        }
    }

    if settings.rofi {
        for result in search_results {
            if result.contains(&media_choice) {
                media_choice = result;
                break;
            }
        }
    }

    let media_info = media_choice.split("\t").collect::<Vec<&str>>();

    let media_id = media_info[1];
    let media_type = media_info[2];
    let media_title = media_info[3].split('[').next().unwrap_or("").trim();

    if media_type == "tv" {
        let show_info = FlixHQ.info(&media_id).await?;

        if let FlixHQInfo::Tv(tv) = show_info {
            let mut seasons: Vec<String> = vec![];

            for season in 0..tv.seasons.total_seasons {
                seasons.push(format!("Season {}", season + 1))
            }

            let season_choice = launcher(
                &vec![],
                settings.rofi,
                &mut RofiArgs {
                    process_stdin: Some(seasons.join("\n")),
                    mesg: Some("Choose a season".to_string()),
                    dmenu: true,
                    case_sensitive: true,
                    entry_prompt: Some("".to_string()),
                    ..Default::default()
                },
                &mut FzfArgs {
                    process_stdin: Some(seasons.join("\n")),
                    reverse: true,
                    delimiter: Some("\t".to_string()),
                    header: Some("Choose a season".to_string()),
                    ..Default::default()
                },
            )
            .await;

            let season_number = season_choice.replace("Season ", "").parse::<usize>()?;

            let mut episodes: Vec<String> = vec![];

            for episode in &tv.seasons.episodes[season_number - 1] {
                episodes.push(episode.title.to_string())
            }

            let episode_choice = launcher(
                &vec![],
                settings.rofi,
                &mut RofiArgs {
                    process_stdin: Some(episodes.join("\n")),
                    mesg: Some("Select an episode:".to_string()),
                    dmenu: true,
                    case_sensitive: true,
                    entry_prompt: Some("".to_string()),
                    ..Default::default()
                },
                &mut FzfArgs {
                    process_stdin: Some(episodes.join("\n")),
                    reverse: true,
                    delimiter: Some("\t".to_string()),
                    header: Some("Select an episode:".to_string()),
                    ..Default::default()
                },
            )
            .await;

            let episode_number = episode_choice
                .strip_prefix("Eps ")
                .and_then(|s| s.split(':').next())
                .unwrap_or("1")
                .trim()
                .parse::<usize>()?;

            let episode_id = &tv.seasons.episodes[season_number - 1][episode_number - 1].id;

            handle_servers(config, settings, episode_id, media_id, media_title).await?;
        }
    } else {
        let episode_id = &media_id.rsplit("-").collect::<Vec<&str>>()[0];

        handle_servers(config, settings, episode_id, media_id, media_title).await?;
    }

    Ok(())
}
