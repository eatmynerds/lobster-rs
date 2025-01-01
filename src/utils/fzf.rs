use crate::utils::SpawnError;
use std::{io::Write, process::Stdio};
use tracing::{debug, error};

pub struct Fzf {
    pub executable: String,
    pub args: Vec<String>,
}

impl Fzf {
    pub fn new() -> Self {
        debug!("Initializing new Fzf instance.");
        Self {
            executable: "fzf".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default, Debug)]
pub struct FzfArgs {
    pub process_stdin: Option<String>,
    pub header: Option<String>,
    pub reverse: bool,
    pub preview: Option<String>,
    pub with_nth: Option<String>,
    pub ignore_case: bool,
    pub query: Option<String>,
    pub cycle: bool,
    pub prompt: Option<String>,
    pub delimiter: Option<String>,
    pub preview_window: Option<String>,
}

pub trait FzfSpawn {
    fn spawn(&mut self, args: &mut FzfArgs) -> Result<std::process::Output, SpawnError>;
}

impl FzfSpawn for Fzf {
    fn spawn(&mut self, args: &mut FzfArgs) -> Result<std::process::Output, SpawnError> {
        let mut temp_args = self.args.clone();

        if let Some(header) = &args.header {
            debug!("Setting header: {}", header);
            temp_args.push(format!("--header={}", header));
        }

        if let Some(prompt) = &args.prompt {
            temp_args.push("--prompt".to_string());
            temp_args.push(prompt.to_string());
        }

        if args.reverse {
            debug!("Adding reverse flag.");
            temp_args.push("--reverse".to_string());
        }

        if let Some(preview) = &args.preview {
            debug!("Setting preview: {}", preview);
            temp_args.push(format!("--preview={}", preview));
        }

        if let Some(with_nth) = &args.with_nth {
            debug!("Setting with-nth: {}", with_nth);
            temp_args.push(format!("--with-nth={}", with_nth));
        }

        if args.ignore_case {
            debug!("Adding ignore-case flag.");
            temp_args.push("--ignore-case".to_string());
        }

        if let Some(query) = &args.query {
            debug!("Setting query: {}", query);
            temp_args.push(format!("--query={}", query));
        }

        if args.cycle {
            debug!("Adding cycle flag.");
            temp_args.push("--cycle".to_string());
        }

        if let Some(delimiter) = &args.delimiter {
            debug!("Setting delimiter: {}", delimiter);
            temp_args.push(format!("--delimiter={}", delimiter));
        }

        if let Some(preview_window) = &args.preview_window {
            debug!("Setting preview-window: {}", preview_window);
            temp_args.push(format!("--preview-window={}", preview_window));
        }

        let mut command = std::process::Command::new(&self.executable);
        command.args(&temp_args);

        debug!("Executing fzf command: {} {:?}", self.executable, temp_args);

        if let Some(process_stdin) = &args.process_stdin {
            debug!("Process stdin provided, writing to stdin.");

            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn().map_err(|e| {
                error!("Failed to spawn process: {}", e);
                SpawnError::IOError(e)
            })?;

            if let Some(mut stdin) = child.stdin.take() {
                writeln!(stdin, "{}", process_stdin).map_err(|e| {
                    error!("Failed to write to stdin: {}", e);
                    SpawnError::IOError(e)
                })?;
            }

            let output = child.wait_with_output().map_err(|e| {
                error!("Failed to wait for process output: {}", e);
                SpawnError::IOError(e)
            })?;

            debug!("Process completed successfully.");
            Ok(output)
        } else {
            debug!("No process stdin provided.");

            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let child = command.spawn().map_err(|e| {
                error!("Failed to spawn process: {}", e);
                SpawnError::IOError(e)
            })?;

            let output = child.wait_with_output().map_err(|e| {
                error!("Failed to wait for process output: {}", e);
                SpawnError::IOError(e)
            })?;

            debug!("Process completed successfully.");
            Ok(output)
        }
    }
}
