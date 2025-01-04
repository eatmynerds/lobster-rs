use crate::flixhq::flixhq::{FlixHQ, FlixHQInfo};
use crate::utils::image_preview::remove_desktop_and_tmp;
use crate::utils::{
    config::Config,
    {
        fzf::FzfArgs,
        rofi::{Rofi, RofiArgs, RofiSpawn},
    },
};
use crate::Args;
use crate::{handle_servers, launcher};
use anyhow::anyhow;
use log::{debug, error, warn};
use std::{io, io::Write, sync::Arc};

pub fn subtitles_prompt() -> bool {
    warn!(
        "Subtitle functionality is unreliable and may significantly slow down video playback since FlixHQ provides incorrect subtitle URLs. (this affects downloading aswell)"
    );

    loop {
        print!("Do you want to try and use subtitles anyway? [y/N]: ");

        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();

        match input {
            "y" => {
                return true;
            }
            "n" => {
                return false;
            }
            _ => {
                println!("Incorrect option. Please enter 'y' or 'n'.");
            }
        }
    }
}

pub fn get_input(rofi: bool) -> anyhow::Result<String> {
    if rofi {
        debug!("Using Rofi interface for input.");

        let mut rofi = Rofi::new();
        debug!("Initializing Rofi with arguments.");

        let rofi_output = match rofi.spawn(&mut RofiArgs {
            sort: true,
            dmenu: true,
            case_sensitive: true,
            width: Some(1500),
            entry_prompt: Some("".to_string()),
            mesg: Some("Search Movie/TV Show".to_string()),
            ..Default::default()
        }) {
            Ok(output) => {
                debug!("Rofi command executed successfully.");
                output
            }
            Err(e) => {
                error!("Failed to execute Rofi command: {}", e);
                return Err(e.into());
            }
        };

        let result = String::from_utf8_lossy(&rofi_output.stdout)
            .trim()
            .to_string();

        debug!("Rofi returned input: {}", result);
        Ok(result)
    } else {
        debug!("Using terminal input for input.");

        print!("Search Movie/TV Show: ");
        if let Err(e) = io::stdout().flush() {
            error!("Failed to flush stdout: {}", e);
            return Err(e.into());
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let result = input.trim().to_string();
                if result.is_empty() {
                    error!("User input is empty.");
                    return Err(anyhow::anyhow!("User input is empty."));
                }
                debug!("User entered input: {}", result);
                Ok(result)
            }
            Err(e) => {
                error!("Failed to read input from stdin: {}", e);
                Err(e.into())
            }
        }
    }
}

pub async fn run(settings: Arc<Args>, config: Arc<Config>) -> anyhow::Result<()> {
    let query = match &settings.query {
        Some(query) => query.to_string(),
        None => get_input(settings.rofi)?,
    };

    let results = FlixHQ.search(&query).await?;
    if results.is_empty() {
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

                let formatted_duration = if movie.duration == "N/A" {
                    "N/A".to_string()
                } else {
                    let movie_duration = movie.duration.replace("m", "").parse::<u32>()?;

                    if movie_duration >= 60 {
                        let hours = movie_duration / 60;
                        let minutes = movie_duration % 60;
                        format!("{}h{}min", hours, minutes)
                    } else {
                        format!("{}m", movie_duration)
                    }
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
                ));
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
                seasons.push(format!("Season {}", season + 1));
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
                episodes.push(episode.title.to_string());
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

            let episode_choices = &tv.seasons.episodes[season_number - 1];

            let result_index = episode_choices
                .iter()
                .position(|episode| episode.title == episode_choice)
                .unwrap_or_else(|| {
                    error!("Invalid episode choice: '{}'", episode_choice);
                    std::process::exit(1);
                });

            let episode_id = &tv.seasons.episodes[season_number - 1][result_index].id;

            handle_servers(config, settings, episode_id, media_id, media_title).await?;
        }
    } else {
        let episode_id = &media_id.rsplit('-').collect::<Vec<&str>>()[0];

        handle_servers(config, settings, episode_id, media_id, media_title).await?;
    }

    Ok(())
}
