use crate::utils::rofi::{Rofi, RofiArgs, RofiSpawn};
use std::{io, io::Write};
use tracing::{debug, error, info};

pub fn get_input(rofi: bool) -> anyhow::Result<String> {
    if rofi {
        info!("Using Rofi interface for input.");

        let mut rofi = Rofi::new();
        debug!("Initializing Rofi with arguments.");

        let rofi_output = match rofi.spawn(&mut RofiArgs {
            sort: true,
            dmenu: true,
            case_sensitive: true,
            width: Some(1500),
            entry_prompt: Some("".to_string()),
            mesg: Some("Search Movie/TV Show".to_string()),
            ..Default::default()
        }) {
            Ok(output) => {
                info!("Rofi command executed successfully.");
                output
            }
            Err(e) => {
                error!("Failed to execute Rofi command: {}", e);
                return Err(e.into());
            }
        };

        let result = String::from_utf8_lossy(&rofi_output.stdout)
            .trim()
            .to_string();

        debug!("Rofi returned input: {}", result);
        Ok(result)
    } else {
        info!("Using terminal input for input.");

        print!("Search Movie/TV Show: ");
        if let Err(e) = io::stdout().flush() {
            error!("Failed to flush stdout: {}", e);
            return Err(e.into());
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let result = input.trim().to_string();
                debug!("User entered input: {}", result);
                Ok(result)
            }
            Err(e) => {
                error!("Failed to read input from stdin: {}", e);
                Err(e.into())
            }
        }
    }
}
