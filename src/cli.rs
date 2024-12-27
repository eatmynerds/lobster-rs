use crate::utils::rofi::{Rofi, RofiArgs, RofiSpawn};
use std::io;
use std::io::Write;

pub fn get_input(rofi: bool) -> Result<String, std::io::Error> {
    if rofi {
        let mut rofi = Rofi::new();

        let output = rofi
            .spawn(RofiArgs {
                sort: true,
                dmenu: true,
                case_sensitive: true,
                width: Some(1500),
                entry_prompt: Some("".to_string()),
                mesg: Some("Search Movie/TV Show".to_string()),
                ..Default::default()
            })
            .unwrap();

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        print!("Search Movie/TV Show: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        Ok(input.trim().to_string())
    }
}
