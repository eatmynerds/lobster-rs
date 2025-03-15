use super::players::mpv::{Mpv, MpvArgs, MpvPlay};
use anyhow::anyhow;
use clap::Parser;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref FILE_PATH: String = if cfg!(windows) {
        std::env::var("LocalAppData").unwrap() + "\\temp\\discord_presence"
    } else {
        String::from("/tmp/discord_presence")
    };
}

const SMALL_IMAGE: &str = "https://images-ext-1.discordapp.net/external/dUSRf56flwFeOMFjafsUhIMMS_1Xs-ptjeDHo6TWn6c/%3Fquality%3Dlossless%26size%3D48/https/cdn.discordapp.com/emojis/1138835294506975262.png";
const PATTERN: &str = r#"(\(Paused\)\s)?AV:\s([0-9:]*) / ([0-9:]*) \(([0-9]*)%\)"#;

pub async fn discord_presence(
    id: &str,
    title: Option<&str>,
    episode_number: Option<usize>,
) -> anyhow::Result<()> {
    let mut client =
        DiscordIpcClient::new(id).map_err(|_| anyhow!("Failed to create discord IPC client!"))?;

    let new_title = title.unwrap_or("No Title");

    client
        .connect()
        .map_err(|_| anyhow!("Failed to connect to discord client!"))?;

    let details = match (title, episode_number) {
        (Some(title), Some(episode_number)) => format!("{} - Episode {}", title, episode_number),
        (Some(title), None) => new_title.to_string(),
        (None, _) => String::from("No Title"),
    };

    let mpv = Mpv::new();

    let mut child = mpv.play(MpvArgs {
        url: String::from("https://b-g-eu-1.raffaellocdn.net:2223/v3-hls-playback/5b195cf64e22d38876f75fb4464f21ba193e5a5083347d8cc79f88c4817c07ddbbc2cb498b1ab6e5f888fff540dd0c215324303d06f496228bb321cb9a0f4dcff50f51cc2d49b6d7a5897239c1eb46cf7eeba5dbe9361cc23b5690136aa698628b769ef8c423b4aa203577de67923c277594e4495f7e01dd5afd470a7d6ad38988256e234d26c584f6d7743bfd402ed4eedf22e328f9fa85d84fd1fb59d709a703fade2f591ce80283bfa90fc95c4691d7a36d5a0e4fcf1f55e1af5d290a9f61/playlist.m3u8"),
        ..Default::default()
    })?;

    let re: regex::Regex = Regex::new(PATTERN).unwrap();

    while child.try_wait()?.is_none() {
        let content = std::fs::read_to_string(&*FILE_PATH)?;
        let captures = re
            .captures_iter(content.as_str())
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

        let episode_text = format!("Episode {}", episode_number.unwrap_or(0));
        let activity = activity::Activity::new()
            .details(details.as_str())
            .state(position.as_str())
            .assets(
                activity::Assets::new()
                    .large_text(&new_title)
                    .small_image(SMALL_IMAGE)
                    .small_text(episode_text.as_str()),
            )
            .buttons(vec![
                activity::Button::new("Github", "https://github.com/justchokingaround/jerry"),
                activity::Button::new("Discord", "https://discord.gg/4P2DaJFxbm"),
            ]);

        client
            .set_activity(activity)
            .map_err(|_| anyhow!("Failed to set new activity!"))?;

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    client
        .close()
        .map_err(|_| anyhow!("Failed to close client connection!"))?;

    Ok(())
}
