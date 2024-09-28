use std::fs;

use anyhow::Context;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    #[error("Serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Custom error: {0}")]
    Custom(String),
}
//anyhow error 的error 能够自动的转换任何的错误，可以简易的进行错误的捕获

//this error，需要通过enum的方式定义error的类型和描述，
// 如error.rs的定义，同时在Result<(),MyError> 需要在可能发生错误的地方通过map_err(|e|MyError:xxx()...)? 进行捕获这个error

// 在main函数中，使用with_context() 进行错误处理，
// 在函数中，使用? 进行错误处理
fn main() -> Result<(), anyhow::Error> {
    println!(
        "size of io::Error: {}",
        std::mem::size_of::<std::io::Error>()
    );
    println!(
        "size of Parse error: {}",
        std::mem::size_of::<std::num::ParseIntError>()
    );
    println!("size of MyError: {}", std::mem::size_of::<MyError>());
    let filename = "data.txt";
    let _fd =
        fs::File::open(filename).with_context(|| format!("can not find file: {}", filename))?;

    fail_with_error()?;
    Ok(())
}

fn fail_with_error() -> Result<(), MyError> {
    Err(MyError::Custom("Failed to read file".to_string()))
}
