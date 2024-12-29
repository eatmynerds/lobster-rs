pub mod config;
pub mod fzf;
pub mod mpv;
pub mod rofi;

#[derive(thiserror::Error, Debug)]
pub enum SpawnError {
    #[error("Failed to spawn process: {0}")]
    IOError(std::io::Error),
}
