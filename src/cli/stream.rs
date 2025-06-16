use log::{debug, error, info};
use std::sync::Arc;
use std::process::Command;
use reqwest::Client;
use regex::Regex;

use crate::{
    Args,
    cli::{cli::{download, player_run_choice}, Quality},
    flixhq::flixhq::{FlixHQ, FlixHQEpisode, FlixHQSourceType, FlixHQSubtitles},
    utils::config::Config,
};

use super::{Languages, Player, Provider};
use crate::utils::{
    SpawnError,
    history::{save_history, save_progress},
    players::{
        celluloid::{Celluloid, CelluloidArgs, CelluloidPlay},
        iina::{Iina, IinaArgs, IinaPlay},
        mpv::{Mpv, MpvArgs, MpvPlay},
        vlc::{Vlc, VlcArgs, VlcPlay},
    },
    presence::discord_presence,
};
use futures::{
    StreamExt,
    future::{BoxFuture, FutureExt},
};
use serde_json::json;

pub async fn handle_servers(
    config: Arc<Config>,
    settings: Arc<Args>,
    next_episode: Option<bool>,
    media_info: (Option<String>, &str, &str, &str, &str),
    show_info: Option<(usize, usize, Vec<Vec<FlixHQEpisode>>)>,
) -> anyhow::Result<()> {
    debug!(
        "Fetching servers for episode_id: {}, media_id: {}",
        media_info.1, media_info.2
    );

    let (episode_id, episode_title, new_show_info, server_results) =
        if let Some(next_episode) = next_episode {
            let show_info = show_info.clone().expect("Failed to get episode info");
            let mut episode_number = show_info.1;
            let mut season_number = show_info.0;

            let total_seasons = show_info.2.len();

            if next_episode {
                let total_episodes = show_info.2[season_number - 1].len();

                if episode_number + 1 < total_episodes {
                    // Move to next episode
                    episode_number += 1;
                } else if season_number < total_seasons {
                    // Move to the first episode of the next season
                    season_number += 1;
                    episode_number = 0;
                } else {
                    // No next episode or season available, staying at the last episode
                    error!("No next episode or season available.");
                    std::process::exit(1);
                }
            } else {
                // Move to the previous episode
                if episode_number > 0 {
                    episode_number -= 1;
                } else if season_number > 1 {
                    // Move to the last episode of the previous season
                    season_number -= 1;
                    episode_number = show_info.2[season_number - 1].len() - 1;
                } else {
                    // No previous episode available, staying at the first episode
                    error!("No previous episode available.");
                    std::process::exit(1);
                }
            }

            let episode_info = show_info.2[season_number - 1][episode_number].clone();

            (
                episode_info.id.clone(),
                Some(episode_info.title),
                Some((season_number, episode_number, show_info.2)),
                FlixHQ
                    .servers(&episode_info.id, media_info.2)
                    .await
                    .map_err(|_| anyhow::anyhow!("Timeout while fetching servers"))?,
            )
        } else {
            (
                media_info.1.to_string(),
                media_info.0,
                show_info,
                FlixHQ
                    .servers(media_info.1, media_info.2)
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
        .sources(episode_id.as_str(), media_info.2, *server)
        .await
        .map_err(|e| anyhow::anyhow!("Timeout while fetching sources: {e}"))?;

    debug!("{}", json!(sources));

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

            debug!("{}", json!(vidcloud_subtitles));

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
                "celluloid" => Player::Celluloid,
                _ => {
                    error!("Player not supported");
                    std::process::exit(1);
                }
            };

            if cfg!(target_os = "android") {
                player = Player::MpvAndroid;
            }

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
                    episode_title,
                    episode_id,
                    media_info.2.to_string(),
                    media_info.3.to_string(),
                    media_info.4.to_string(),
                ),
                new_show_info.map(|(a, b, c)| (a, b, c)),
                selected_subtitles,
                Some(settings.language.unwrap_or(Languages::English)),
            )
            .await?;
        }
    }

    Ok(())
}

async fn url_quality(url: String, quality: Option<Quality>) -> anyhow::Result<String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let input = client.get(url).send().await?.text().await?;

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

struct MediaInfo {}

pub fn handle_stream(
    settings: Arc<Args>,
    config: Arc<Config>,
    player: Player,
    download_dir: Option<String>,
    url: String,
    media_info: (Option<String>, String, String, String, String),
    episode_info: Option<(usize, usize, Vec<Vec<FlixHQEpisode>>)>,
    subtitles: Vec<String>,
    subtitle_language: Option<Languages>,
) -> BoxFuture<'static, anyhow::Result<()>> {
    let subtitles_choice = settings.no_subs;
    let player_url = url.clone();

    let subtitles_for_player = if subtitles_choice {
        info!("Continuing without subtitles");
        None
    } else {
        if subtitles.len() > 0 {
            Some(subtitles.clone())
        } else {
            info!("No subtitles available!");
            None
        }
    };

    let subtitle_language = if subtitles_choice {
        subtitle_language
    } else {
        None
    };

    async move {
        match player {
            Player::Celluloid => {
                if let Some(download_dir) = download_dir {
                    download(
                        download_dir,
                        media_info.3,
                        url,
                        subtitles_for_player,
                        subtitle_language,
                    )
                    .await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let title = if let Some(title) = media_info.0 {
                    format!("{} - {}", media_info.3, title)
                } else {
                    media_info.3
                };

                let celluloid = Celluloid::new();

                celluloid.play(CelluloidArgs {
                    url,
                    mpv_sub_files: subtitles_for_player,
                    mpv_force_media_title: Some(title),
                    ..Default::default()
                })?;
            }
            Player::Iina => {
                if let Some(download_dir) = download_dir {
                    download(
                        download_dir,
                        media_info.3,
                        url,
                        subtitles_for_player,
                        subtitle_language,
                    )
                    .await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let title = if let Some(title) = media_info.0 {
                    format!("{} - {}", media_info.3, title)
                } else {
                    media_info.3
                };

                let iina = Iina::new();

                iina.play(IinaArgs {
                    url,
                    no_stdin: true,
                    keep_running: true,
                    mpv_sub_files: subtitles_for_player,
                    mpv_force_media_title: Some(title),
                    ..Default::default()
                })?;
            }
            Player::Vlc => {
                if let Some(download_dir) = download_dir {
                    download(
                        download_dir,
                        media_info.3,
                        url,
                        subtitles_for_player,
                        subtitle_language,
                    )
                    .await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let url = url_quality(url, settings.quality).await?;

                let title: String = if let Some(title_part) = &media_info.0 {
                    format!("{} - {}", media_info.3, title_part)
                } else {
                    media_info.3.to_string()
                };

                let vlc = Vlc::new();

                vlc.play(VlcArgs {
                    url,
                    input_slave: subtitles_for_player,
                    meta_title: Some(title),
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
                        media_info.3,
                        url,
                        subtitles_for_player.clone(),
                        subtitle_language,
                    )
                    .await?;

                    info!("Download completed. Exiting...");
                    return Ok(());
                }

                let watchlater_path =
                    format!("{}/lobster-rs/watchlater", std::env::temp_dir().display());

                let watchlater_dir = std::path::PathBuf::new().join(&watchlater_path);

                if watchlater_dir.exists() {
                    std::fs::remove_dir_all(&watchlater_dir)
                        .expect("Failed to remove watchlater directory!");
                }

                std::fs::create_dir_all(&watchlater_dir)
                    .expect("Failed to create watchlater directory!");

                let url = url_quality(url, settings.quality).await?;

                let title: String = if let Some(title_part) = &media_info.0 {
                    format!("{} - {}", media_info.3, title_part)
                } else {
                    media_info.3.to_string()
                };

                let mpv = Mpv::new();

                let mut child = mpv.play(MpvArgs {
                    url: url.clone(),
                    sub_files: subtitles_for_player.clone(),
                    force_media_title: Some(title),
                    watch_later_dir: Some(watchlater_path),
                    write_filename_in_watch_later_config: true,
                    save_position_on_quit: true,
                    ..Default::default()
                })?;

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

                if config.history {
                    let (position, progress) = save_progress(url).await?;

                    save_history(media_info.clone(), episode_info.clone(), position, progress)
                        .await?;
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
            Player::MpvAndroid => {
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

                let title: String = if let Some(title_part) = media_info.0 {
                    format!("{} - {}", media_info.3, title_part)
                } else {
                    media_info.3.to_string()
                };

                Command::new("am")
                    .args([
                        "start",
                        "--user",
                        "0",
                        "-a",
                        "android.intent.action.VIEW",
                        "-d",
                        &url,
                        "-n",
                        "is.xyz.mpv/.MPVActivity",
                        "-e",
                        "title",
                        &title,
                    ])
                    .spawn()
                    .map_err(|e| {
                        error!("Failed to start MPV for Android: {}", e);
                        SpawnError::IOError(e)
                    })?;
            }
            Player::SyncPlay => {
                let url = url_quality(url, settings.quality).await?;

                let title: String = if let Some(title_part) = media_info.0 {
                    format!("{} - {}", media_info.3, title_part)
                } else {
                    media_info.3.to_string()
                };

                Command::new("syncplay")
                    .args([&url, "--", &format!("--force-media-title={}", title)])
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
