use std::{thread, time::Duration};

use anyhow::Result;
use tokio::sync::mpsc;
#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel(32);
    let handler = worker(rx);
    tokio::spawn(async move {
        let mut i = 0;
        loop {
            i += 1;
            let send = format!("Future {}", i);
            println!("send {} ", send);
            tx.send(send.to_string()).await?;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    handler.join().unwrap();
    Ok(())
}

fn worker(mut rx: mpsc::Receiver<String>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while let Some(msg) = rx.blocking_recv() {
            let ret = expensive_blocking_task(&msg);
            println!("result :{}", ret);
        }
    })
}

fn expensive_blocking_task(name: &str) -> String {
    // println!("{} is running", name);
    thread::sleep(Duration::from_secs(1));
    blake3::hash(name.as_bytes()).to_string()
}
