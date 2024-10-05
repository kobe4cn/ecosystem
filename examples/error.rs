use std::fs::File;
use std::io::{self, Read};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
}

fn open_file_and_parse(path: &str) -> Result<i32, MyError> {
    let mut file = File::open(path).map_err(|_| MyError::FileNotFound(path.to_string()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let number = contents.trim().parse::<i32>()?;
    Ok(number)
}

fn main() {
    match open_file_and_parse("data.txt") {
        Ok(num) => println!("Parsed number: {}", num),
        Err(e) => println!("Error: {}", e),
    }
}
