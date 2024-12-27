use super::SpawnError;

pub struct Rofi {
    pub executable: String,
    pub args: Vec<String>,
}

impl Rofi {
    pub fn new() -> Self {
        Self {
            executable: "rofi".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default)]
pub struct RofiArgs {
    mesg: Option<String>,
    filter: Option<String>,
    sort: bool,
    show_icons: bool,
    show: Option<String>,
    drun_categories: Option<String>,
    theme: Option<String>,
    dmenu: bool,
    case_sensitive: bool,
    width: Option<u32>,
    left_display_prompt: Option<String>,
    entry_prompt: Option<String>,
    display_columns: Option<u32>,
}

pub trait RofiSpawn {
    fn spawn(&mut self, args: RofiArgs) -> Result<std::process::Child, SpawnError>;
}

impl RofiSpawn for Rofi {
    fn spawn(&mut self, args: RofiArgs) -> Result<std::process::Child, SpawnError> {
        let mut temp_args = self.args.clone();

        if let Some(filter) = args.filter {
            temp_args.push("-filter".to_string());
            temp_args.push(filter);
        }

        if args.show_icons {
            temp_args.push("-show-icons".to_string());
        }

        if let Some(drun_categories) = args.drun_categories {
            temp_args.push("-drun-categories".to_string());
            temp_args.push(drun_categories);
        }

        if let Some(theme) = args.theme {
            temp_args.push("-theme".to_string());
            temp_args.push(theme);
        }

        if args.sort {
            temp_args.push("-sort".to_string());
        }

        if args.dmenu {
            temp_args.push("-dmenu".to_string());
        }

        if args.case_sensitive {
            temp_args.push("-i".to_string());
        }

        if let Some(width) = args.width {
            temp_args.push("-width".to_string());
            temp_args.push(width.to_string());
        }

        if let Some(show) = args.show {
            temp_args.push("-show".to_string());
            temp_args.push(show);
        }

        if let Some(left_display_prompt) = args.left_display_prompt {
            temp_args.push("-left-display-prompt".to_string());
            temp_args.push(left_display_prompt);
        }

        if let Some(entry_prompt) = args.entry_prompt {
            temp_args.push("-p".to_string());
            temp_args.push(entry_prompt);
        }

        if let Some(display_columns) = args.display_columns {
            temp_args.push("-display-columns".to_string());
            temp_args.push(display_columns.to_string());
        }

        if let Some(mesg) = args.mesg {
            temp_args.push("-mesg".to_string());
            temp_args.push(mesg);
        }

        std::process::Command::new(&self.executable)
            .args(temp_args)
            .spawn()
            .map_err(SpawnError::IOError)
    }
}

#[cfg(test)]
mod test {
    use crate::utils::rofi::{Rofi, RofiArgs, RofiSpawn};

    #[test]
    fn test_rofi_spawn() {
        let args = RofiArgs {
            sort: true,
            dmenu: true,
            case_sensitive: true,
            entry_prompt: Some("".to_string()),
            mesg: Some("Hello\nWorld!".to_string()),
            ..Default::default()
        };

        let mut rofi = Rofi::new();
        let mut child = rofi.spawn(args).unwrap();

        assert_eq!(
            child
                .wait()
                .expect("Failed to spawn child process for mpv.")
                .code(),
            Some(0)
        )
    }
}
