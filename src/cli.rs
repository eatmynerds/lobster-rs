use crate::utils::rofi::{Rofi, RofiArgs, RofiSpawn};
use std::{io, io::Write};

pub fn get_input(rofi: bool) -> anyhow::Result<String> {
    if rofi {
        let mut rofi = Rofi::new();

        let rofi_output = rofi.spawn(RofiArgs {
            sort: true,
            dmenu: true,
            case_sensitive: true,
            width: Some(1500),
            entry_prompt: Some("".to_string()),
            mesg: Some("Search Movie/TV Show".to_string()),
            ..Default::default()
        })?;

        Ok(String::from_utf8_lossy(&rofi_output.stdout)
            .trim()
            .to_string())
    } else {
        print!("Search Movie/TV Show: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        Ok(input.trim().to_string())
    }
}
