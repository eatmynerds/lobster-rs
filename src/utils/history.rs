use crate::flixhq::flixhq::FlixHQEpisode;
use anyhow::anyhow;
use reqwest::Client;
use std::fs::OpenOptions;
use std::io::prelude::*;

pub async fn save_progress(url: String) -> anyhow::Result<(String, f32)> {
    let watchlater_dir = std::path::PathBuf::new().join(format!(
        "{}/lobster-rs/watchlater",
        std::env::temp_dir().display()
    ));

    let mut durations: Vec<f32> = vec![];

    let re = regex::Regex::new(r#"#EXTINF:([0-9]*\.?[0-9]+),"#).unwrap();

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(url).send().await?.text().await?;

    for capture in re.captures_iter(&response) {
        if let Some(duration) = capture.get(1) {
            durations.push(duration.as_str().parse::<f32>().unwrap());
        }
    }

    let entries: Vec<_> = std::fs::read_dir(watchlater_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .collect();

    let file_path = entries[0].path();

    let watchlater_contents = std::fs::read_to_string(&file_path)?;

    let start_pos = watchlater_contents.split("start=").collect::<Vec<&str>>()[1].trim();

    let position = start_pos
        .chars()
        .position(|i| i == '\n')
        .map(|n| &start_pos[..n])
        .unwrap_or_else(|| start_pos);

    let position = position.parse::<f32>().unwrap();

    let total_duration: f32 = durations.iter().sum();

    let progress = (position * 100.0) / total_duration;

    let new_position = format!(
        "{:.2}:{:.2}:{:.2}",
        (position / 3600.0),
        (position / 60.0 % 60.0),
        (position % 60.0)
    );

    Ok((new_position, progress))
}

fn write_to_history(info: String) -> anyhow::Result<()> {
    let history_file_dir = dirs::data_local_dir()
        .expect("Failed to find local dir")
        .join("lobster-rs");

    if !history_file_dir.exists() {
        std::fs::create_dir_all(&history_file_dir)?;
    }

    let history_file = history_file_dir.join("lobster_history.txt");

    if !history_file.exists() {
        std::fs::File::create(&history_file)?;
    }

    let mut file = OpenOptions::new().append(true).open(history_file).unwrap();
    if let Err(e) = writeln!(file, "{}", info) {
        eprintln!("Couldn't write to file: {}", e);
    }

    Ok(())
}

fn remove_from_history(media_id: String) -> anyhow::Result<()> {
    let history_file_dir = dirs::data_local_dir()
        .expect("Failed to find local dir")
        .join("lobster-rs");

    if !history_file_dir.exists() {
        std::fs::create_dir_all(&history_file_dir)?;
    }

    let history_file = history_file_dir.join("lobster_history.txt");

    if !history_file.exists() {
        return Err(anyhow!("History file does not exist!"));
    }

    let mut history_file_temp = std::fs::read_to_string(&history_file)?
        .lines()
        .map(String::from)
        .collect::<Vec<String>>();

    if let Some(pos) = history_file_temp.iter().position(|x| x.contains(&media_id)) {
        let _ = history_file_temp.remove(pos);
    } else {
        return Err(anyhow!("Episode does not exist in history file yet!"));
    }

    std::fs::write(history_file, history_file_temp.join("\n"))?;

    Ok(())
}

pub async fn save_history(
    media_info: (Option<String>, String, String, String, String),
    episode_info: Option<(usize, usize, Vec<Vec<FlixHQEpisode>>)>,
    position: String,
    progress: f32,
) -> anyhow::Result<()> {
    let media_type = media_info.2.split('/').collect::<Vec<&str>>()[0];

    match media_type {
        "movie" => {
            if progress > 90.0 {
                if remove_from_history(media_info.2.clone()).is_ok() {
                } else {
                    write_to_history(format!(
                        "{}\t{}\t{}\t{}",
                        media_info.3, position, media_info.2, media_info.4
                    ))?;
                }

                return Ok(());
            }

            write_to_history(format!(
                "{}\t{}\t{}\t{}",
                media_info.3, position, media_info.2, media_info.4
            ))?;
        }
        "tv" => {
            if let Some((mut season_number, mut episode_number, episodes)) = episode_info {
                if progress > 90.0 {
                    episode_number += 1;

                    if episode_number >= episodes[season_number - 1].len() {
                        if season_number < episodes.len() {
                            season_number += 1;
                            episode_number = 0;
                        }
                    }

                    if remove_from_history(media_info.2.clone()).is_ok() {
                    } else {
                        write_to_history(format!(
                            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                            media_info.3,
                            position,
                            media_info.2,
                            media_info.1,
                            season_number,
                            episodes[season_number - 1][episode_number].title,
                            media_info.4
                        ))?;
                    }

                    return Ok(());
                }

                write_to_history(format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    media_info.3,
                    position,
                    media_info.2,
                    media_info.1,
                    season_number,
                    episodes[season_number - 1][episode_number].title,
                    media_info.4
                ))?;
            }
        }
        _ => return Err(anyhow!("Unknown media type!")),
    }

    Ok(())
}
