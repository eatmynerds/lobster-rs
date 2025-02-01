use crate::CLIENT;

async fn save_progress(url: String, media_title: String) -> anyhow::Result<()> {
    let watchlater_dir = std::path::PathBuf::new().join("/tmp/lobster-rs/watchlater");

    if watchlater_dir.exists() {
        std::fs::remove_dir_all(&watchlater_dir).expect("Failed to remove watchlater directory!");
    }

    std::fs::create_dir_all(&watchlater_dir).expect("Failed to create watchlater directory!");

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

    let position = watchlater_contents.split("start=").collect::<Vec<&str>>()[1]
        .trim()
        .parse::<f32>()
        .unwrap();

    let new_position = format!(
        "{:.2}:{:.2}:{:.2}",
        (position / 3600.0),
        (position / 60.0 % 60.0),
        (position % 60.0)
    );

    println!(
        "{}",
        format!(
            "{} {} {}",
            media_title,
            std::path::PathBuf::new()
                .join("/tmp/lobster-images")
                .display(),
            new_position
        )
    );

    Ok(())
}
