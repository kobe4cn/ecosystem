use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, PartialEq, Deserialize, Debug)]
struct User {
    name: String,
    age: u8,
    skills: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum MyState {
    Init(String),
    Running(Vec<String>),
    Done(u32),
}

fn main() -> Result<()> {
    let user = User {
        name: "John".to_string(),
        age: 20,
        skills: vec!["Rust".to_string(), "Python".to_string()],
    };
    let serialized = serde_json::to_string(&user)?;
    let user1 = serde_json::from_str(&serialized)?;
    println!("user1 :{:?}", user1);
    assert_eq!(user, user1);

    let state = MyState::Running(vec!["Rust".to_string(), "Python".to_string()]);
    let serialized = serde_json::to_string(&state)?;
    println!("serialized :{}", serialized);
    Ok(())
}
