pub mod fzf;
pub mod rofi;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum SpawnError {
    IOError(std::io::Error),
}

impl Error for SpawnError {}

impl Display for SpawnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}
