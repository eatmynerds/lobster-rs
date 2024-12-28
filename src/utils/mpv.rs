use super::SpawnError;
use std::io::Write;

pub struct Mpv {
    pub executable: String,
    pub args: Vec<String>,
}

impl Mpv {
    pub fn new() -> Self {
        Self {
            executable: "mpv".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default)]
pub struct MpvArgs {
    pub url: String,
    pub subtitle: Option<String>,
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
    fn play(&self, args: MpvArgs) -> Result<std::process::Child, SpawnError>;
}

impl MpvPlay for Mpv {
    fn play(&self, args: MpvArgs) -> Result<std::process::Child, SpawnError> {
        let mut temp_args = self.args.clone();

        temp_args.push(args.url);

        if args.quiet {
            temp_args.push(String::from("--quiet"));
        }

        if args.really_quiet {
            temp_args.push(String::from("--really-quiet"));
        }

        if let Some(msg_level) = args.msg_level {
            temp_args.push(format!("--msg-level=all={}", msg_level));
        }

        if args.save_position_on_quit {
            temp_args.push(String::from("--save-position-on-quit"));
        }

        if args.write_filename_in_watch_later_config {
            temp_args.push(String::from("--write-filename-in-watch-later-config"));
        }

        if let Some(watch_later_dir) = args.watch_later_dir {
            temp_args.push(format!("--watch-later-dir={}", watch_later_dir));
        }

        if let Some(input_ipc_server) = args.input_ipc_server {
            temp_args.push(format!("--input-ipc-server={}", input_ipc_server));
        }

        if let Some(subtitle) = args.subtitle {
            temp_args.push(format!("--sub-file={subtitle}"));
        }

        if let Some(force_media_title) = args.force_media_title {
            temp_args.push(format!("--force-media-title={}", force_media_title));
        }

        std::process::Command::new(&self.executable)
            .args(temp_args)
            .spawn()
            .map_err(SpawnError::IOError)
    }
}

#[cfg(test)]
mod test {
    use crate::utils::mpv::{Mpv, MpvArgs, MpvPlay};

    #[test]
    fn test_mpv_spawn() {
        let mpv = super::Mpv::new();

        let mut child = mpv
            .play(MpvArgs {
                url: String::from("https://www.youtube.com/watch?v=sNHzizPu7yQ&t=1s"),
                msg_level: Some(String::from("no")),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(
            child
                .wait()
                .expect("Failed to spawn child process for mpv.")
                .code(),
            Some(0)
        )
    }
}