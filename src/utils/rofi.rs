use crate::utils::SpawnError;
use tracing::{debug, error, info};
use std::io::Write;

pub struct Rofi {
    executable: String,
    pub args: Vec<String>,
}

impl Rofi {
    pub fn new() -> Self {
        info!("Initializing new Rofi instance.");
        Self {
            executable: "rofi".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default, Debug)]
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
    fn spawn(&mut self, args: &mut RofiArgs) -> Result<std::process::Output, SpawnError>;
}

impl RofiSpawn for Rofi {
    fn spawn(&mut self, args: &mut RofiArgs) -> Result<std::process::Output, SpawnError> {
        let mut temp_args = self.args.clone();

        debug!("Preparing arguments for Rofi execution.");
        if let Some(filter) = &args.filter {
            temp_args.push("-filter".to_string());
            temp_args.push(filter.to_string());
            debug!("Added filter argument: {}", filter);
        }

        if args.show_icons {
            temp_args.push("-show-icons".to_string());
            debug!("Enabled show-icons.");
        }

        if let Some(drun_categories) = &args.drun_categories {
            temp_args.push("-drun-categories".to_string());
            temp_args.push(drun_categories.to_string());
            debug!("Added drun-categories: {}", drun_categories);
        }

        if let Some(theme) = &args.theme {
            temp_args.push("-theme".to_string());
            temp_args.push(theme.to_string());
            debug!("Added theme: {}", theme);
        }

        if args.sort {
            temp_args.push("-sort".to_string());
            debug!("Enabled sorting.");
        }

        if args.dmenu {
            temp_args.push("-dmenu".to_string());
            debug!("Enabled dmenu mode.");
        }

        if args.case_sensitive {
            temp_args.push("-i".to_string());
            debug!("Enabled case sensitivity.");
        }

        if let Some(width) = &args.width {
            temp_args.push("-width".to_string());
            temp_args.push(width.to_string());
            debug!("Set width to {}", width);
        }

        if let Some(show) = &args.show {
            temp_args.push("-show".to_string());
            temp_args.push(show.to_string());
            debug!("Set show mode to {}", show);
        }

        if let Some(left_display_prompt) = &args.left_display_prompt {
            temp_args.push("-left-display-prompt".to_string());
            temp_args.push(left_display_prompt.to_string());
            debug!("Set left display prompt: {}", left_display_prompt);
        }

        if let Some(entry_prompt) = &args.entry_prompt {
            temp_args.push("-p".to_string());
            temp_args.push(entry_prompt.to_string());
            debug!("Set entry prompt: {}", entry_prompt);
        }

        if let Some(display_columns) = &args.display_columns {
            temp_args.push("-display-columns".to_string());
            temp_args.push(display_columns.to_string());
            debug!("Set display columns to {}", display_columns);
        }

        if let Some(mesg) = &args.mesg {
            temp_args.push("-mesg".to_string());
            temp_args.push(mesg.to_string());
            debug!("Set message: {}", mesg);
        }

        let mut command = std::process::Command::new(&self.executable);
        command.args(&temp_args);

        debug!("Constructed command: {:?}", command);

        if let Some(process_stdin) = &args.process_stdin {
            info!("Spawning Rofi process with stdin.");
            command
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let mut child = command.spawn().map_err(|e| {
                error!("Failed to spawn Rofi process: {}", e);
                SpawnError::IOError(e)
            })?;

            if let Some(mut stdin) = child.stdin.take() {
                debug!("Writing to stdin: {}", process_stdin);
                writeln!(stdin, "{}", process_stdin).map_err(|e| {
                    error!("Failed to write to stdin: {}", e);
                    SpawnError::IOError(e)
                })?;
            }

            let output = child.wait_with_output().map_err(|e| {
                error!("Failed to wait for Rofi process: {}", e);
                SpawnError::IOError(e)
            })?;

            info!("Rofi process completed successfully.");
            Ok(output)
        } else {
            info!("Spawning Rofi process without stdin.");
            command
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let child = command.spawn().map_err(|e| {
                error!("Failed to spawn Rofi process: {}", e);
                SpawnError::IOError(e)
            })?;

            let output = child.wait_with_output().map_err(|e| {
                error!("Failed to wait for Rofi process: {}", e);
                SpawnError::IOError(e)
            })?;

            info!("Rofi process completed successfully.");
            Ok(output)
        }
    }
}
