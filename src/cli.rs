use crate::flixhq::flixhq::{FlixHQ, FlixHQInfo};
use crate::utils::image_preview::remove_desktop_and_tmp;
use crate::utils::{
    config::Config,
    {
        fzf::FzfArgs,
        rofi::{Rofi, RofiArgs, RofiSpawn},
    },
};
use crate::{handle_servers, launcher};
use crate::{Args, MediaType};
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
    if settings.r#continue {
        let history_file = dirs::data_local_dir()
            .expect("Failed to find local dir")
            .join("lobster-rs/lobster_history.txt");

        if !history_file.exists() {
            error!("History file not found!");
            std::process::exit(1)
        }

        let history_text = std::fs::read_to_string(history_file).unwrap();

        let mut history_choices: Vec<String> = vec![];
        let mut history_image_files: Vec<(String, String, String)> = vec![];
        let history_entries = history_text.split("\n").collect::<Vec<&str>>();
        for (i, history_entry) in history_entries.iter().enumerate() {
            if i == history_entries.len() - 1 {
                break;
            }

            let entries = history_entry.split("\t").collect::<Vec<&str>>();
            let title = entries[0];
            let media_type = entries[2].split('/').collect::<Vec<&str>>()[0];
            match media_type {
                "tv" => {
                    let temp_episode = entries[5].replace(":", "");

                    let episode_number = temp_episode
                        .split_whitespace()
                        .nth(1)
                        .expect("Failed to parse episode number from history!");

                    if settings.image_preview {
                        history_image_files.push((
                            format!("{} {} {}", title, entries[4], entries[5]),
                            entries[6].to_string(),
                            entries[3].to_string(),
                        ))
                    }

                    history_choices.push(format!(
                        "{} (tv) Season {} {}\t{}\t{}\t{}\t{}\t{}\t{}",
                        title,
                        entries[4],
                        entries[5],
                        entries[3],
                        entries[2],
                        entries[6],
                        entries[4],
                        episode_number,
                        title,
                    ))
                }
                "movie" => {
                    let episode_id = entries[2].rsplit("-").collect::<Vec<&str>>()[0];

                    if settings.image_preview {
                        history_image_files.push((
                            title.to_string(),
                            entries[3].to_string(),
                            entries[2].to_string(),
                        ))
                    }

                    history_choices.push(format!(
                        "{} (movie)\t{}\t{}\t{}",
                        title, episode_id, entries[2], entries[3]
                    ))
                }
                _ => {}
            }
        }

        let history_choice = launcher(
            &history_image_files,
            settings.rofi,
            &mut RofiArgs {
                mesg: Some("Choose an entry: ".to_string()),
                process_stdin: Some(history_choices.join("\n")),
                dmenu: true,
                case_sensitive: true,
                entry_prompt: Some("".to_string()),
                display_columns: Some(1),
                ..Default::default()
            },
            &mut FzfArgs {
                prompt: Some("Choose an entry: ".to_string()),
                process_stdin: Some(history_choices.join("\n")),
                reverse: true,
                with_nth: Some("1".to_string()),
                delimiter: Some("\t".to_string()),
                ..Default::default()
            },
        )
        .await;

        let entry = history_choice.split("\t").collect::<Vec<&str>>();
        let media_type = entry[2].split('/').collect::<Vec<&str>>()[0];
        match media_type {
            "tv" => {
                let show_info = FlixHQ.info(entry[2]).await?;
                if let FlixHQInfo::Tv(tv) = show_info {
                    let season_number = entry[4]
                        .parse::<usize>()
                        .expect("Failed to parse season number!");
                    let episode_number = entry[5]
                        .parse::<usize>()
                        .expect("Failed to parse episode number!");
                    handle_servers(
                        config.clone(),
                        settings.clone(),
                        (entry[1], entry[2], entry[6], entry[3]),
                        Some((season_number, episode_number, tv.seasons.episodes)),
                    )
                    .await?;
                }
            }
            "movie" => {
                handle_servers(
                    config.clone(),
                    settings.clone(),
                    (entry[1], entry[2], entry[0], entry[3]),
                    None,
                )
                .await?
            }
            _ => {}
        }
    }

    let results = if let Some(recent) = &settings.recent {
        match recent {
            MediaType::Movie => FlixHQ.recent_movies().await?,
            MediaType::Tv => FlixHQ.recent_shows().await?,
        }
    } else if let Some(trending) = &settings.trending {
        match trending {
            MediaType::Movie => FlixHQ.trending_movies().await?,
            MediaType::Tv => FlixHQ.trending_shows().await?,
        }
    } else {
        let query = match &settings.query {
            Some(query) => query.to_string(),
            None => get_input(settings.rofi)?,
        };

        FlixHQ.search(&query).await?
    };

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
    let media_image = media_info[0];
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

            let episode_number = episode_choices
                .iter()
                .position(|episode| episode.title == episode_choice)
                .unwrap_or_else(|| {
                    error!("Invalid episode choice: '{}'", episode_choice);
                    std::process::exit(1);
                });

            let episode_id = tv.seasons.episodes[season_number - 1][episode_number]
                .id
                .clone();

            handle_servers(
                config,
                settings,
                (&episode_id, media_id, media_title, media_image),
                Some((season_number, episode_number, tv.seasons.episodes)),
            )
            .await?;
        }
    } else {
        let episode_id = &media_id.rsplit('-').collect::<Vec<&str>>()[0];

        handle_servers(
            config,
            settings,
            (episode_id, media_id, media_title, media_image),
            None,
        )
        .await?;
    }

    Ok(())
}
