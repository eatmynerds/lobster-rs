use std::io;
use std::io::Write;

pub fn get_input() -> Result<String, std::io::Error> {
    print!("Search Movie/TV Show: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}
