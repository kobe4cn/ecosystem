use std::{
    thread::{self},
    time::Duration,
};

use tokio::{fs, runtime::Builder, time::sleep};

fn main() {
    let handle = thread::spawn(|| {
        //execute a future
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.spawn(async {
            println!("Future1!");
            let content = fs::read_to_string("Cargo.toml").await.unwrap();
            println!("content len:{}", content.len());
        });
        rt.spawn(async {
            println!("Future2!");
            let ret = expensive_blocking_task("Future2");
            println!("Future2 done, hash:{}", ret);
        });
        rt.block_on(async {
            sleep(Duration::from_secs(15)).await;
            println!("hi number from the spawned thread!");
        });
    });

    println!("hi number from the main thread!");

    handle.join().unwrap();
}

fn expensive_blocking_task(name: &str) -> String {
    println!("{} is running", name);
    thread::sleep(Duration::from_secs(1));
    blake3::hash(name.as_bytes()).to_string()
}
