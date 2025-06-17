use crate::utils::SpawnError;
use ctrlc;
use log::{debug, error};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct Vlc {
    pub executable: String,
    pub args: Vec<String>,
}

impl Vlc {
    pub fn new() -> Self {
        debug!("Initializing new vlc instance.");
        Self {
            executable: "vlc".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default, Debug)]
pub struct VlcArgs {
    pub url: String,
    pub input_slave: Option<Vec<String>>,
    pub meta_title: Option<String>,
}

pub trait VlcPlay {
    fn play(&self, args: VlcArgs) -> Result<(), SpawnError>;
}

impl VlcPlay for Vlc {
    fn play(&self, args: VlcArgs) -> Result<(), SpawnError> {
        debug!("Preparing to play video with URL: {:?}", args.url);

        let mut temp_args = self.args.clone();
        temp_args.push(args.url.clone());

        if let Some(input_slave) = &args.input_slave {
            let input_slave_arg = format!(r#"--input-slave="{}""#, input_slave.join("#"));
            temp_args.push(input_slave_arg.clone());
            debug!("Added input-slave argument: {}", input_slave_arg);
        }

        if let Some(meta_title) = &args.meta_title {
            let meta_title_arg = format!("--meta-title={}", meta_title);
            temp_args.push(meta_title_arg.clone());
            debug!("Added meta-title argument: {}", meta_title_arg);
        }

        debug!(
            "Executing VLC command: {} with args: {:?}",
            self.executable, temp_args
        );

        debug!("Executing mpv command: {} {:?}", self.executable, temp_args);

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
                error!("Failed to spawn VLC process: {}", e);
                SpawnError::IOError(e)
            })?;

        Ok(())
    }
}
