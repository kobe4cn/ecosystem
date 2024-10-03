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

const MAX_MESSAGE_SIZE: usize = 128;
#[tokio::main]
async fn main() -> Result<()> {
    // console_subscriber::init(); can not work with tracing_subscriber, so use the following code to replace it
    //using consoleLayer builder spawn to make a layer and register with tracing_subscriber
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

#[derive(Debug, Default)]
struct State {
    peers: DashMap<SocketAddr, mpsc::Sender<Arc<Message>>>,
}

#[derive(Debug)]
enum Message {
    Join(String),
    Leave(String),
    Chat { sender: String, content: String },
}
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
        Self::Chat { sender, content }
    }
}
#[derive(Debug)]
struct Peer {
    username: String,
    stream: SplitStream<Framed<TcpStream, LinesCodec>>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Join(message) => write!(f, "[{}]", message),
            Self::Leave(message) => write!(f, "[{}]", message),
            Self::Chat { sender, content } => write!(f, "{}: {}", sender, content),
        }
    }
}

impl State {
    async fn broadcast(&self, addr: SocketAddr, message: Arc<Message>) {
        for peer in self.peers.iter() {
            if peer.key() != &addr {
                if let Err(e) = peer.value().send(message.clone()).await {
                    warn!("failed to send message to peer {}: {}", peer.key(), e);
                    //remove the peer from the state if the message fails to send
                    self.peers.remove(peer.key());
                }
            }
        }
    }

    async fn add(
        &self,
        addr: SocketAddr,
        username: String,
        stream: Framed<TcpStream, LinesCodec>,
    ) -> Peer {
        let (tx, mut rx) = mpsc::channel(MAX_MESSAGE_SIZE);
        self.peers.insert(addr, tx);
        let (mut stream_sender, stream_receiver) = stream.split();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = stream_sender.send(message.to_string()).await {
                    warn!("failed to send message to peer {}: {}", addr, e);
                    break;
                }
            }
        });
        Peer {
            username,
            stream: stream_receiver,
        }
    }
}

async fn handle_connection(socket: TcpStream, addr: SocketAddr, state: Arc<State>) -> Result<()> {
    info!("new connection from {}", addr);
    let mut stream = Framed::new(socket, LinesCodec::new());
    stream.send("Enter your username:").await?;

    let username = match stream.next().await {
        Some(Ok(username)) => username,
        Some(Err(e)) => return Err(anyhow::anyhow!("failed to read username: {}", e)),
        None => return Ok(()),
    };
    let mut peer = state.add(addr, username, stream).await;
    //notify others that a new user has joined
    let message = Arc::new(Message::user_join(peer.username.clone()));
    info!("{}", message);
    state.broadcast(addr, message).await;
    while let Some(message) = peer.stream.next().await {
        let message = match message {
            Ok(message) => message,
            Err(e) => {
                warn!("failed to read message: {}", e);
                break;
            }
        };
        let message = Arc::new(Message::chat(peer.username.clone(), message));
        state.broadcast(addr, message).await;
    }
    //notify others that a user has left
    state.peers.remove(&addr);
    let message = Arc::new(Message::user_leave(peer.username.clone()));
    state.broadcast(addr, message).await;

    info!("peer {} left", peer.username);
    Ok(())
}
