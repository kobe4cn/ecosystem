use std::{fmt, net::SocketAddr, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;
use futures::{stream::SplitStream, SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
//控制读取消息的大小
const MAX_MESSAGE_SIZE: usize = 1024;
//消息类型定义，包含多种消息类型，如用户加入、离开、发送消息
#[derive(Debug)]
enum Message {
    Join(String),
    Leave(String),
    Msg(String),
}
//实现消息类型的显示方法，如果不实现Display trait for Message,
//则无法使用{}打印消息,同时.to_string()方法也会报错
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Join(message) => write!(f, "[{}]", message),
            Self::Leave(message) => write!(f, "[{}]", message),
            Self::Msg(message) => write!(f, "{}", message),
        }
    }
}
//实现消息类型的构造方法, 设定消息的内容
impl Message {
    fn user_join(username: String) -> Self {
        let message = format!("{} has joined the chat", username);
        Self::Join(message)
    }
    fn user_leave(username: String) -> Self {
        let message = format!("{} has left the chat", username);
        Self::Leave(message)
    }
    fn chat(sender: String, content: String) -> Self {
        let message = format!("{}: {}", sender, content);
        Self::Msg(message)
    }
}

#[derive(Debug, Default)]
struct State {
    clients: DashMap<SocketAddr, mpsc::Sender<Arc<Message>>>,
}
#[derive(Debug)]
struct Client {
    username: String,
    stream: SplitStream<Framed<TcpStream, LinesCodec>>,
}
#[tokio::main]
async fn main() -> Result<()> {
    let console_layer = console_subscriber::ConsoleLayer::builder().spawn();
    let layer = Layer::new().pretty().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry()
        .with(console_layer)
        .with(layer)
        .init();

    let addr = "0.0.0.0:8080";
    info!("listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    let state = Arc::new(State::default());
    loop {
        let (socket, addr) = listener.accept().await?;
        let state_cloned = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, addr, state_cloned).await {
                warn!("failed to handle connection: {}", e);
            }
        });
    }
}
impl State {
    async fn add_client(
        &self,
        addr: SocketAddr,
        username: String,
        stream: Framed<TcpStream, LinesCodec>,
    ) -> Client {
        let (tx, mut rx) = mpsc::channel(MAX_MESSAGE_SIZE);
        self.clients.insert(addr, tx);
        let (mut stream_sender, stream_receiver) = stream.split();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = stream_sender.send(message.to_string()).await {
                    warn!("failed to send message to peer {}: {}", addr, e);
                    break;
                }
            }
        });
        Client {
            username,
            stream: stream_receiver,
        }
    }

    async fn broadcast(&self, addr: SocketAddr, message: Arc<Message>) {
        for client in self.clients.iter() {
            if client.key() != &addr {
                if let Err(e) = client.value().send(message.clone()).await {
                    warn!("failed to send message to peer {}: {}", client.key(), e);
                    //remove the peer from the state if the message fails to send
                    self.clients.remove(client.key());
                }
            }
        }
    }
}

async fn handle_connection(socket: TcpStream, addr: SocketAddr, state: Arc<State>) -> Result<()> {
    let codec = LinesCodec::new();
    let mut stream = Framed::new(socket, codec);
    stream.send("Enter name: ").await?;

    let username = match stream.next().await {
        Some(Ok(username)) => username,
        Some(Err(e)) => return Err(anyhow::anyhow!("failed to read username: {}", e)),
        None => return Ok(()),
    };
    let mut client = state.add_client(addr, username, stream).await;
    let message = Arc::new(Message::user_join(client.username.clone()));
    state.broadcast(addr, message).await;
    while let Some(message) = client.stream.next().await {
        let message = match message {
            Ok(message) => message,
            Err(e) => {
                warn!("failed to read message: {}", e);
                break;
            }
        };
        let message = Arc::new(Message::chat(client.username.clone(), message));
        state.broadcast(addr, message).await;
    }
    state.clients.remove(&addr);
    let message = Arc::new(Message::user_leave(client.username.clone()));
    state.broadcast(addr, message).await;
    info!("peer {} left", client.username);
    Ok(())
}
