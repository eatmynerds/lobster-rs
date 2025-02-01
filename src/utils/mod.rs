pub mod config;
pub mod ffmpeg;
pub mod fzf;
pub mod image_preview;
pub mod players;
pub mod rofi;
pub mod history;

#[derive(thiserror::Error, Debug)]
pub enum SpawnError {
    #[error("Failed to spawn process: {0}")]
    IOError(std::io::Error),
}
