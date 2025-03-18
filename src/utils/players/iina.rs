use crate::utils::SpawnError;
use log::{debug, error};

pub struct Iina {
    pub executable: String,
    pub args: Vec<String>,
}

impl Iina {
    pub fn new() -> Self {
        debug!("Initializing new iina instance.");
        Self {
            executable: "iina".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default, Debug)]
pub struct IinaArgs {
    pub url: String,
    pub no_stdin: bool,
    pub keep_running: bool,
    pub mpv_sub_files: Option<Vec<String>>,
    pub mpv_force_media_title: Option<String>,
}

pub trait IinaPlay {
    fn play(&self, args: IinaArgs) -> Result<(), SpawnError>;
}

impl IinaPlay for Iina {
    fn play(&self, args: IinaArgs) -> Result<(), SpawnError> {
        debug!("Preparing to play video with URL: {:?}", args.url);

        let mut temp_args = self.args.clone();
        temp_args.push(args.url.clone());

        if args.no_stdin {
            temp_args.push("--no-stdin".to_string());
        }

        if args.keep_running {
            temp_args.push("--keep-running".to_string());
        }

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
