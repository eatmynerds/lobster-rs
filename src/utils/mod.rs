pub mod config;
pub mod decrypt;
pub mod ffmpeg;
pub mod fzf;
pub mod history;
pub mod image_preview;
pub mod players;
pub mod presence;
pub mod rofi;

#[derive(thiserror::Error, Debug)]
pub enum SpawnError {
    #[error("Failed to spawn process: {0}")]
    IOError(std::io::Error),
}
