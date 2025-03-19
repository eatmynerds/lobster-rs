use crate::utils::SpawnError;
use log::{debug, error};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Celluloid {
    pub executable: String,
    pub args: Vec<String>,
}

impl Celluloid {
    pub fn new() -> Self {
        debug!("Initializing new celluloid instance.");
        Self {
            executable: "celluloid".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default, Debug)]
pub struct CelluloidArgs {
    pub url: String,
    pub mpv_sub_files: Option<Vec<String>>,
    pub mpv_force_media_title: Option<String>,
}

pub trait CelluloidPlay {
    fn play(&self, args: CelluloidArgs) -> Result<(), SpawnError>;
}

impl CelluloidPlay for Celluloid {
    fn play(&self, args: CelluloidArgs) -> Result<(), SpawnError> {
        debug!("Preparing to play video with URL: {:?}", args.url);

        let mut temp_args = self.args.clone();
        temp_args.push(args.url.clone());

        if let Some(mpv_sub_files) = args.mpv_sub_files {
            let temp_sub_files = mpv_sub_files
                .iter()
                .map(|sub_file| sub_file.replace(":", r#"\:"#))
                .collect::<Vec<_>>()
                .join(":");

            temp_args.push(format!("--mpv-sub-files={}", temp_sub_files));
        }

        if let Some(mpv_force_media_title) = args.mpv_force_media_title {
            temp_args.push(format!("--mpv-force-media-title={}", mpv_force_media_title));
        }

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        match ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        }) {
            Ok(_) => {}
            Err(_) => {}
        }

        std::process::Command::new(&self.executable)
            .args(temp_args)
            .status()
            .map_err(|e| {
                error!("Failed to spawn iina process: {}", e);
                SpawnError::IOError(e)
            })?;

        Ok(())
    }
}
