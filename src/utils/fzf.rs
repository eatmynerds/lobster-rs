use super::SpawnError;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    io::{Read, Write},
    process::Stdio,
};

pub struct Fzf {
    pub executable: String,
    pub args: Vec<String>,
}

impl Fzf {
    pub fn new() -> Self {
        Self {
            executable: "fzf".to_string(),
            args: vec![],
        }
    }
}

#[derive(Default)]
pub struct FzfArgs {
    pub print_query: Option<String>,
    pub header: Option<String>,
    pub reverse: bool,
    pub preview: Option<String>,
    pub with_nth: Option<String>,
    pub ignore_case: bool,
    pub query: Option<String>,
    pub cycle: bool,
    pub delimiter: Option<String>,
    pub preview_window: Option<String>,
}

pub trait FzfSpawn {
    fn spawn(&mut self, args: FzfArgs) -> Result<std::process::Output, SpawnError>;
}

impl FzfSpawn for Fzf {
    fn spawn(&mut self, args: FzfArgs) -> Result<std::process::Output, SpawnError> {
        let mut temp_args = self.args.clone();

        if let Some(header) = args.header {
            temp_args.push(format!("--header={}", header));
        }

        if args.reverse {
            temp_args.push("--reverse".to_string());
        }

        if let Some(preview) = args.preview {
            temp_args.push(format!("--preview={}", preview));
        }

        if let Some(with_nth) = args.with_nth {
            temp_args.push(format!("--with-nth={}", with_nth));
        }

        if args.ignore_case {
            temp_args.push("--ignore-case".to_string());
        }

        if let Some(query) = args.query {
            temp_args.push(format!("--query={}", query));
        }

        if args.cycle {
            temp_args.push("--cycle".to_string());
        }

        if let Some(delimiter) = args.delimiter {
            temp_args.push(format!("--delimiter={}", delimiter));
        }

        if let Some(preview_window) = args.preview_window {
            temp_args.push(format!("--preview-window={}", preview_window));
        }

        let mut command = std::process::Command::new(&self.executable);
        command.args(&temp_args);

        if let Some(print_query) = args.print_query {
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn().map_err(SpawnError::IOError)?;

            if let Some(mut stdin) = child.stdin.take() {
                writeln!(stdin, "{}", print_query).map_err(SpawnError::IOError)?;
            }

            let output = child.wait_with_output().map_err(SpawnError::IOError)?;

            Ok(output)
        } else {
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn().map_err(SpawnError::IOError)?;

            let output = child.wait_with_output().map_err(SpawnError::IOError)?;

            Ok(output)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::fzf::{Fzf, FzfArgs, FzfSpawn};

    #[test]
    fn test_fzf_spawn() {
        let args = FzfArgs {
            print_query: Some("Hello\nWorld".to_string()),
            delimiter: Some(String::from("\t")),
            ..Default::default()
        };

        let mut fzf = Fzf::new();
        let output = fzf.spawn(args).unwrap();

        assert_eq!(output.status.success(), true);
    }
}
