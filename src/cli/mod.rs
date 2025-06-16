pub mod cli;
pub mod stream;

use std::{fmt::Display, num::ParseIntError, str::FromStr};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, IntoStaticStr};
use thiserror::Error;
use log::error;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    AsRefStr,
    IntoStaticStr,
    Display,
    EnumIter,
)]
#[non_exhaustive]
pub enum Player {
    #[default]
    Mpv,
    Vlc,
    Iina,
    Celluloid,
    MpvAndroid,
    SyncPlay,
}


#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum PlayerError {
    #[error("the inputted name did not correspond to a known video player")]
    InvalidPlayer { player_name: String },
}

impl FromStr for Player {
    type Err = PlayerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mpv" => Ok(Player::Mpv),
            "vlc" => Ok(Player::Vlc),
            "iina" => Ok(Player::Iina),
            "celluloid" => Ok(Player::Celluloid),
            "mpvandroid" => Ok(Player::MpvAndroid),
            "syncplay" => Ok(Player::SyncPlay),
            _ => Err(PlayerError::InvalidPlayer {
                player_name: s.to_string(),
            }),
        }
    }
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, Copy, PartialEq)]
#[clap(rename_all = "PascalCase")]
pub enum Provider {
    Vidcloud,
    Upcloud,
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Vidcloud => write!(f, "Vidcloud"),
            Provider::Upcloud => write!(f, "Upcloud"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum Quality {
    #[clap(name = "360")]
    Q360 = 360,
    #[clap(name = "720")]
    Q720 = 720,
    #[clap(name = "1080")]
    Q1080 = 1080,
}

#[derive(thiserror::Error, Debug)]
pub enum StreamError {
    #[error("Failed to parse quality from string: {0}")]
    QualityParseError(
        #[from]
        #[source]
        ParseIntError,
    ),
}

impl FromStr for Quality {
    type Err = StreamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let quality = s.parse::<u32>()?;
        Ok(match quality {
            0..=600 => Quality::Q360,
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_u32())
    }
}

#[derive(ValueEnum, Debug, Clone, Serialize, Deserialize, Copy, Default)]
#[clap(rename_all = "PascalCase")]
pub enum Languages {
    Arabic,
    Turkish,
    Danish,
    Dutch,
    #[default]
    English,
    Finnish,
    German,
    Italian,
    Russian,
    Spanish,
}

impl Display for Languages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Languages::Arabic => write!(f, "Arabic"),
            Languages::Turkish => write!(f, "Turkish"),
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
