use anyhow::anyhow;
use clap::{Parser, ValueEnum};
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter},
    io::Write,
    num::ParseIntError,
    path::PathBuf,
    str::FromStr,
};

mod cli;
use cli::get_input;
mod flixhq;
use flixhq::{search::FlixHQInfo, FlixHQ};
mod utils;
use utils::{
    fzf::{Fzf, FzfArgs, FzfSpawn},
    rofi::{Rofi, RofiArgs, RofiSpawn},
    ProcessArgs,
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

#[derive(ValueEnum, Debug, Clone)]
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

#[derive(ValueEnum, Debug, Clone)]
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
    #[clap(short, long, default_value_t = false)]
    r#continue: bool,

    /// Downloads movie or episode that is selected (defaults to current directory)
    #[clap(short, long, default_value = ".")]
    download: PathBuf,

    /// Enables discord rich presence (beta feature, works fine on Linux)
    #[clap(short, long, default_value_t = false)]
    rpc: bool,

    /// Edit config file using an editor defined with lobster_editor in the config ($EDITOR by default)
    #[clap(short, long, default_value_t = false)]
    edit: bool,

    /// Shows image previews during media selection
    #[clap(short, long, default_value_t = false)]
    image_preview: bool,

    /// Outputs JSON containing video links, subtitle links, etc.
    #[clap(short, long, default_value_t = false)]
    json: bool,

    /// Specify the subtitle language
    #[clap(short, long, default_value = "English")]
    language: Languages,

    /// Use rofi instead of fzf
    #[clap(long, default_value_t = false)]
    rofi: bool,

    /// Specify the provider to watch from
    #[clap(short, long, value_enum, default_value = "Vidcloud")]
    provider: Provider,

    /// Specify the video quality
    #[clap(short, long, value_enum, default_value = "q1080")]
    quality: Quality,

    /// Lets you select from the most recent movies or TV shows
    #[clap(long, value_enum, default_value = "movie")]
    recent: MediaType,

    /// Use Syncplay to watch with friends
    #[clap(short, long, default_value_t = false)]
    syncplay: bool,

    /// Lets you select from the most popular movies and shows
    #[clap(short, long, default_value_t = false)]
    trending: bool,

    /// Update the script
    #[clap(short, long, default_value_t = false)]
    update: bool,

    /// Enable debug mode (prints debug info to stdout and saves it to $TEMPDIR/lobster.log)
    #[clap(long, default_value_t = false)]
    debug: bool,
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.download.to_str() {
            Some(path) => write!(f, "{}", path),
            None => write!(f, ""),
        }
    }
}

fn fzf_launcher(args: FzfArgs) -> String {
    let mut fzf = Fzf::new();

    let output = fzf.spawn(args).unwrap();

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn rofi_launcher(args: RofiArgs) -> String {
    let mut rofi = Rofi::new();

    let output = rofi.spawn(args).unwrap();

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn launcher(args: ProcessArgs) -> String {
    match args {
        ProcessArgs::Fzf(fzf_args) => fzf_launcher(fzf_args),
        ProcessArgs::Rofi(rofi_args) => rofi_launcher(rofi_args),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let query = match args.query {
        Some(query) => query,
        None => get_input(args.rofi)?,
    };

    let results = FlixHQ.search(&query).await?;

    if results.len() == 0 {
        return Err(anyhow!("No results found"));
    }

    let mut search_results: Vec<String> = vec![];

    for result in results {
        match result {
            FlixHQInfo::Movie(movie) => search_results.push(format!(
                "{}\t{}\t{}\t{} [{}] [{}]",
                movie.image, movie.id, movie.media_type, movie.title, movie.year, movie.duration
            )),
            FlixHQInfo::Tv(tv) => search_results.push(format!(
                "{}\t{}\t{}\t{} [SZNS {}] [EPS {}]",
                tv.image, tv.id, tv.media_type, tv.title, tv.seasons, tv.episodes
            )),
        }
    }

    let media_choice = if args.rofi {
        launcher(ProcessArgs::Rofi(RofiArgs {
            process_stdin: Some(search_results.join("\n")),
            mesg: Some("Choose a movie or TV show".to_string()),
            dmenu: true,
            case_sensitive: true,
            entry_prompt: Some("".to_string()),
            display_columns: Some(4),
            ..Default::default()
        }))
    } else {
        launcher(ProcessArgs::Fzf(FzfArgs {
            process_stdin: Some(search_results.join("\n")),
            reverse: true,
            with_nth: Some("4,5,6,7,8".to_string()),
            delimiter: Some("\t".to_string()),
            header: Some("Choose a movie or TV show".to_string()),
            ..Default::default()
        }))
    };

    let media_info = media_choice.split("\t").collect::<Vec<&str>>();

    let image_link = media_info[0];
    let media_id = media_info[1];
    let media_type = media_info[2];
    let media_title = media_info[3].split('[').next().unwrap_or("").trim();

    if media_type == "tv" {
        let show_info = FlixHQ.info(&media_id).await?;

        if let FlixHQInfo::Tv(tv) = show_info {
            let mut seasons: Vec<String> = vec![];

            for season in 0..tv.seasons {
                seasons.push(format!("Season {}", season + 1))
            }

            let season_choice = if args.rofi {
                launcher(ProcessArgs::Rofi(RofiArgs {
                    process_stdin: Some(seasons.join("\n")),
                    mesg: Some("Choose a season".to_string()),
                    dmenu: true,
                    case_sensitive: true,
                    entry_prompt: Some("".to_string()),
                    ..Default::default()
                }))
            } else {
                launcher(ProcessArgs::Fzf(FzfArgs {
                    process_stdin: Some(seasons.join("\n")),
                    reverse: true,
                    delimiter: Some("\t".to_string()),
                    header: Some("Choose a season".to_string()),
                    ..Default::default()
                }))
            };

            println!("Selected season: {}", season_choice);
        }
    }

    Ok(())
}
