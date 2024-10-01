use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::{extract::State, routing::patch, Json};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::{format::FmtSpan, Layer},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer as _,
};

#[derive(Serialize, PartialEq, Deserialize, Debug, Clone)]
struct User {
    name: String,
    age: u8,
    skill: Vec<String>,
}

#[derive(Serialize, PartialEq, Deserialize, Debug, Clone)]
struct UpdateUser {
    age: Option<u8>,
    skill: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .pretty()
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry().with(layer).init();

    let user = User {
        name: "John".to_string(),
        age: 30,
        skill: vec!["Rust".to_string(), "Python".to_string()],
    };
    let user = Arc::new(Mutex::new(user));

    let addr = "0.0.0.0:8081";
    info!("listening on {}", addr);
    let app = axum::Router::new()
        .route("/", axum::routing::get(index_handler))
        .route("/", patch(update_handler))
        .with_state(user);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
#[tracing::instrument]
async fn index_handler(State(user): State<Arc<Mutex<User>>>) -> Json<User> {
    (*user.lock().unwrap()).clone().into()
}

#[tracing::instrument]
async fn update_handler(
    State(user): State<Arc<Mutex<User>>>,
    Json(update_user): Json<UpdateUser>,
) -> Json<User> {
    let mut user = user.lock().unwrap();
    if let Some(age) = update_user.age {
        user.age = age;
    }
    if let Some(skill) = update_user.skill {
        user.skill = skill;
    }
    Json(user.clone())
}
