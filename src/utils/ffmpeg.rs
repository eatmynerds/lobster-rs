use std::sync::{Arc, atomic::AtomicBool};

use crate::utils::SpawnError;
use log::{debug, error};

pub struct Ffmpeg {
    pub executable: String,
    pub args: Vec<String>,
}

impl Ffmpeg {
    pub fn new() -> Self {
        debug!("Initializing new ffmpeg instance.");
        Self {
            executable: "ffmpeg".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default)]
pub struct FfmpegArgs<'a> {
    pub input_file: String,
    pub stats: bool,
    pub log_level: Option<String>,
    pub output_file: String,
    pub subtitle_files: Option<&'a Vec<String>>,
    pub subtitle_language: Option<String>,
    pub codec: Option<String>,
}

pub trait FfmpegSpawn {
    fn embed_video(&self, args: FfmpegArgs) -> Result<(), SpawnError>;
}

impl FfmpegSpawn for Ffmpeg {
    fn embed_video(&self, args: FfmpegArgs) -> Result<(), SpawnError> {
        debug!("Starting embed_video with input file: {}", args.input_file);

        let mut temp_args = self.args.clone();
        temp_args.push("-i".to_string());
        temp_args.push(args.input_file.to_owned());

        if args.stats {
            debug!("Adding stats flag.");
            temp_args.push("-stats".to_string());
        }

        if let Some(log_level) = &args.log_level {
            debug!("Setting log level to: {}", log_level);
            temp_args.push("-loglevel".to_string());
            temp_args.push(log_level.to_owned());
        }

        if let Some(subtitle_files) = args.subtitle_files {
            let subtitle_count = subtitle_files.len();
            debug!("Embedding {} subtitle files.", subtitle_count);

            if subtitle_count > 1 {
                for subtitle_file in subtitle_files {
                    debug!("Adding subtitle file: {}", subtitle_file);
                    temp_args.push("-i".to_string());
                    temp_args.push(subtitle_file.to_string());
                }

                temp_args.extend("-map 0:v -map 0:a".split(" ").map(String::from));

                for i in 1..=subtitle_count {
                    temp_args.push("-map".to_string());
                    temp_args.push(i.to_string());
                }

                temp_args.extend("-c:v copy -c:a copy -c:s srt".split(" ").map(String::from));

                for i in 1..=subtitle_count {
                    let metadata = format!(
                        "-metadata:s:s:{} language={}_{}",
                        i - 1,
                        args.subtitle_language.as_deref().unwrap_or("English"),
                        i
                    );
                    debug!("Adding metadata: {}", metadata);
                    temp_args.push(metadata);
                }
            } else {
                temp_args.push("-i".to_string());
                temp_args.push(subtitle_files.join("\n"));
                temp_args.extend("-map 0:v -map 0:a -map 1".split(" ").map(String::from));
                temp_args.push("-metadata:s:s:0".to_string());
                let language = format!(
                    "language={}",
                    args.subtitle_language.as_deref().unwrap_or("English")
                );
                debug!("Adding single subtitle metadata: {}", language);
                temp_args.push(language);
            }
        }

        if let Some(codec) = &args.codec {
            debug!("Setting codec to: {}", codec);
            temp_args.push("-c".to_string());
            temp_args.push(codec.to_string());
        }

        temp_args.push(args.output_file.to_owned());
        debug!("Output file set to: {}", args.output_file);

        debug!(
            "Executing ffmpeg command: {} {:?}",
            self.executable, temp_args
        );

        let running = Arc::new(AtomicBool::new(true));

        let r = running.clone();

        match ctrlc::set_handler(move || {
            r.store(false, std::sync::atomic::Ordering::SeqCst);
        }) {
            Ok(_) => {}
            Err(_) => {}
        }

        let exit_status = std::process::Command::new(&self.executable)
            .args(temp_args)
            .status()
            .map_err(|e| {
                error!("Error executing ffmpeg command: {}", e);
                std::process::exit(1);
            })?;

        if exit_status.code() != Some(0) {
            error!("Failed to download {:?}", args.output_file);
            std::process::exit(1);
        }

        Ok(())
    }
}
