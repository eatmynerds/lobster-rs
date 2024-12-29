use crate::utils::SpawnError;
use std::io::Write;

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
    pub process_stdin: Option<String>,
    pub mesg: Option<String>,
    pub filter: Option<String>,
    pub sort: bool,
    pub show_icons: bool,
    pub show: Option<String>,
    pub drun_categories: Option<String>,
    pub theme: Option<String>,
    pub dmenu: bool,
    pub case_sensitive: bool,
    pub width: Option<u32>,
    pub left_display_prompt: Option<String>,
    pub entry_prompt: Option<String>,
    pub display_columns: Option<u32>,
}

pub trait RofiSpawn {
    fn spawn(&mut self, args: RofiArgs) -> Result<std::process::Output, SpawnError>;
}

impl RofiSpawn for Rofi {
    fn spawn(&mut self, args: RofiArgs) -> Result<std::process::Output, SpawnError> {
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

        let mut command = std::process::Command::new(&self.executable);
        command.args(&temp_args);

        if let Some(process_stdin) = args.process_stdin {
            command
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let mut child = command.spawn().map_err(SpawnError::IOError)?;

            if let Some(mut stdin) = child.stdin.take() {
                writeln!(stdin, "{}", process_stdin).map_err(SpawnError::IOError)?;
            }

            let output = child.wait_with_output().map_err(SpawnError::IOError)?;

            Ok(output)
        } else {
            command
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let child = command.spawn().map_err(SpawnError::IOError)?;

            let output = child.wait_with_output().map_err(SpawnError::IOError)?;

            Ok(output)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::rofi::{Rofi, RofiArgs, RofiSpawn};

    #[test]
    fn test_rofi_spawn() {
        let mut rofi = Rofi::new();
        let output = rofi
            .spawn(RofiArgs {
                process_stdin: Some("Hello\nWorld!".to_string()),
                sort: true,
                dmenu: true,
                case_sensitive: true,
                entry_prompt: Some("".to_string()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(output.status.success(), true);
    }
}
