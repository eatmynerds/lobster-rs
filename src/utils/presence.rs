use anyhow::anyhow;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use lazy_static::lazy_static;
use regex::Regex;
use std::{io::{Cursor, Read}, process::Child};

lazy_static! {
    static ref FILE_PATH: String = if cfg!(windows) {
        std::env::var("LocalAppData").unwrap() + "\\temp\\discord_presence"
    } else {
        String::from("/tmp/discord_presence")
    };
}

const PATTERN: &str = r#"(\(Paused\)\s)?AV:\s([0-9:]*) / ([0-9:]*) \(([0-9]*)%\)"#;

pub async fn discord_presence(
    title: &str,
    season_and_episode_num: Option<(usize, usize)>,
    mut mpv_child: Child,
    large_image: &str,
) -> anyhow::Result<()> {
    let mut client =
        DiscordIpcClient::new("1340948447305535592").map_err(|_| anyhow!("Failed to create discord IPC client!"))?;

    client
        .connect()
        .map_err(|_| anyhow!("Failed to connect to discord client!"))?;

    let details = match season_and_episode_num {
        Some((season_num, episode_num)) => format!("{} - Season {} Episode {}", title, season_num, episode_num),
        None => title.to_string(),
    };    

    let re: regex::Regex = Regex::new(PATTERN).unwrap();

    let mut output = mpv_child.stdout.take().unwrap();
    let buffer = vec![0; 256];
    let mut cursor = Cursor::new(buffer);
    while mpv_child.try_wait()?.is_none() {
        cursor.set_position(0);
        let offset = cursor.position();
        let bread = output.read(&mut cursor.get_mut()[offset as usize..])?;
        cursor.set_position(offset + bread as u64);
        let read_data = &cursor.get_ref()[..cursor.position() as usize];

        let content = String::from_utf8_lossy(&read_data);

        let captures = re
            .captures_iter(&content)
            .last()
            .ok_or("Could not match the regex pattern.");

        let position = match captures {
            Ok(captures) => {
                let (_paused, av_first, av_second, _percentage) = (
                    captures.get(1).map_or("", |m| m.as_str()),
                    captures.get(2).map_or("", |m| m.as_str()),
                    captures.get(3).map_or("", |m| m.as_str()),
                    captures.get(4).map_or("", |m| m.as_str()),
                );
                format!("{}/{}", av_first, av_second)
            }
            Err(_) => String::from(""),
        };

        let activity = activity::Activity::new()
            .details(details.as_str())
            .state(position.as_str())
            .assets(
                activity::Assets::new()
                    .large_image(large_image)
                    .large_text(&title)
            )
            .buttons(vec![
                activity::Button::new("Github", "https://github.com/eatmynerds/lobster-rs"),
                activity::Button::new("Discord", "https://discord.gg/4P2DaJFxbm"),
            ]);

        client
            .set_activity(activity)
            .map_err(|_| anyhow!("Failed to set new activity!"))?;
    }

    client
        .close()
        .map_err(|_| anyhow!("Failed to close client connection!"))?;

    Ok(())
}
