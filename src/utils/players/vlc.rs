use crate::utils::SpawnError;

pub struct Vlc {
    pub executable: String,
    pub args: Vec<String>,
}

impl Vlc {
    pub fn new() -> Self {
        Self {
            executable: "vlc".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default)]
pub struct VlcArgs {
    pub url: String,
    pub input_slave: Option<Vec<String>>,
    pub meta_title: Option<String>,
}

pub trait VlcPlay {
    fn play(&self, args: VlcArgs) -> Result<std::process::Child, SpawnError>;
}

impl VlcPlay for Vlc {
    fn play(&self, args: VlcArgs) -> Result<std::process::Child, SpawnError> {
        let mut temp_args = self.args.clone();

        temp_args.push(args.url);

        if let Some(input_slave) = args.input_slave {
            temp_args.push(format!(r#"--input-slave="{}""#, input_slave.join("#")));
        }

        if let Some(meta_title) = args.meta_title {
            temp_args.push(format!("--meta-title={}", meta_title));
        }

        std::process::Command::new(&self.executable)
            .args(temp_args)
            .spawn()
            .map_err(SpawnError::IOError)
    }
}

#[cfg(test)]
mod test {
    use crate::utils::players::vlc::{Vlc, VlcArgs, VlcPlay};

    #[test]
    fn test_vlc_spawn() {
        let vlc = Vlc::new();

        let mut child = vlc
            .play(VlcArgs {
                url: String::from("https://www.youtube.com/watch?v=sNHzizPu7yQ&t=1s"),
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
