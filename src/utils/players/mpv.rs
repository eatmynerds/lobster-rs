use crate::utils::SpawnError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use log::{debug, error};

pub struct Mpv {
    pub executable: String,
    pub args: Vec<String>,
}

impl Mpv {
    pub fn new() -> Self {
        debug!("Initializing new mpv instance.");
        Self {
            executable: "mpv".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default, Debug)]
pub struct MpvArgs {
    pub url: String,
    pub sub_file: Option<String>,
    pub sub_files: Option<Vec<String>>,
    pub force_media_title: Option<String>,
    pub quiet: bool,
    pub really_quiet: bool,
    pub save_position_on_quit: bool,
    pub write_filename_in_watch_later_config: bool,
    pub watch_later_dir: Option<String>,
    pub input_ipc_server: Option<String>,
    pub msg_level: Option<String>,
}

pub trait MpvPlay {
    fn play(&self, args: MpvArgs) -> Result<(), SpawnError>;
}

impl MpvPlay for Mpv {
    fn play(&self, args: MpvArgs) -> Result<(), SpawnError> {
        debug!("Preparing to play video with URL: {:?}", args.url);

        let mut temp_args = self.args.clone();
        temp_args.push(args.url.clone());

        if args.quiet {
            debug!("Adding quiet flag");
            temp_args.push(String::from("--quiet"));
        }

        if args.really_quiet {
            debug!("Adding really quiet flag");
            temp_args.push(String::from("--really-quiet"));
        }

        if let Some(msg_level) = args.msg_level {
            debug!("Setting message level: {}", msg_level);
            temp_args.push(format!("--msg-level=all={}", msg_level));
        }

        if let Some(sub_files) = args.sub_files {
            let temp_sub_files = sub_files
                .iter()
                .map(|sub_file| sub_file.replace(":", r#"\:"#))
                .collect::<Vec<_>>()
                .join(":");

            debug!("Adding subtitle files: {}", temp_sub_files);
            temp_args.push(format!("--sub-files={}", temp_sub_files));
        }

        if args.save_position_on_quit {
            debug!("Adding save position on quit flag");
            temp_args.push(String::from("--save-position-on-quit"));
        }

        if args.write_filename_in_watch_later_config {
            debug!("Adding write filename in watch later config flag");
            temp_args.push(String::from("--write-filename-in-watch-later-config"));
        }

        if let Some(watch_later_dir) = args.watch_later_dir {
            debug!("Setting watch later directory: {}", watch_later_dir);
            temp_args.push(format!("--watch-later-dir={}", watch_later_dir));
        }

        if let Some(input_ipc_server) = args.input_ipc_server {
            debug!("Setting input IPC server: {}", input_ipc_server);
            temp_args.push(format!("--input-ipc-server={}", input_ipc_server));
        }

        if let Some(sub_file) = args.sub_file {
            debug!("Adding subtitle file: {}", sub_file);
            temp_args.push(format!("--sub-file={sub_file}"));
        }

        if let Some(force_media_title) = args.force_media_title {
            debug!("Forcing media title: {}", force_media_title);
            temp_args.push(format!("--force-media-title={}", force_media_title));
        }

        debug!("Executing mpv command: {} {:?}", self.executable, temp_args);

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        std::process::Command::new(&self.executable)
            .args(temp_args)
            .status()
            .map_err(|e| {
                error!("Failed to spawn MPV process: {}", e);
                SpawnError::IOError(e)
            })?;

        Ok(())
    }
}
