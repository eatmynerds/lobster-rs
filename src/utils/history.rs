use crate::CLIENT;
use std::fs::OpenOptions;
use std::io::prelude::*;

pub async fn save_progress(url: String) -> anyhow::Result<(String, f32)> {
    let watchlater_dir = std::path::PathBuf::new().join("/tmp/lobster-rs/watchlater");

    let mut durations: Vec<f32> = vec![];

    let re = regex::Regex::new(r#"#EXTINF:([0-9]*\.?[0-9]+),"#).unwrap();

    let response = CLIENT.get(url).send().await?.text().await?;

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

fn write_to_history(info: String) {
    std::fs::write(
        history_file_dir.join("lobster_history.txt"),
        format!(
            "{}\t{}\t{}\t{}\n",
            media_info.1, position, media_info.0, media_info.2
        ),
    )?;
    let mut file = OpenOptions::new().append(true).open("my-file").unwrap();

    if let Err(e) = writeln!(file, "A new line!") {
        eprintln!("Couldn't write to file: {}", e);
    }
}

pub async fn save_history(
    media_info: (String, String, String),
    episode_info: Option<(String, String, String)>,
    position: String,
    progress: f32,
) -> anyhow::Result<()> {
    let media_type = media_info.0.split('/').collect::<Vec<&str>>()[0];

    let history_file_dir = dirs::data_local_dir()
        .expect("Failed to find local dir")
        .join("lobster-rs");

    if !history_file_dir.exists() {
        std::fs::create_dir_all(&history_file_dir)?;
    }

    match media_type {
        "movie" => {
            if progress > 90.0 {
                // TODO: Remove the movie from the history file
                todo!()
            }
        }
        "tv" => {
            if progress > 90.0 {
                todo!()
            }

            println!("{:#?}", episode_info);
        }
        _ => {
            todo!()
        }
    }

    Ok(())
}
