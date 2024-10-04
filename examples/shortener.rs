use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use http::{header::LOCATION, HeaderMap, StatusCode};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool};
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{
    fmt::{format::FmtSpan, Layer},
    layer::SubscriberExt,
    util::SubscriberInitExt as _,
    Layer as _,
};
#[derive(Debug)]
struct AppState {
    db: PgPool,
}
#[derive(Debug, Deserialize)]
struct ShortenRequest {
    url: String,
}
#[derive(Debug, FromRow)]
struct UrlRecord {
    #[sqlx(default)]
    id: String,
    #[sqlx(default)]
    url: String,
}
#[derive(Debug, Serialize)]
struct ShortenResponse {
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .pretty()
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry().with(layer).init();
    let state =
        Arc::new(AppState::new("postgres://postgres:postgres@localhost:5432/shortener").await?);
    info!("Connecting to database");
    let addr = "0.0.0.0:9876";

    let app = axum::Router::new()
        .route("/", post(shorten))
        .route("/:id", get(redirect))
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", addr);
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

async fn shorten(
    State(state): State<Arc<AppState>>,
    Json(data): Json<ShortenRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let id = state.shorten(&data.url).await.map_err(|e| {
        warn!("Failed to shorten url: {}", e);
        StatusCode::UNPROCESSABLE_ENTITY
    })?;
    let body = Json(ShortenResponse {
        url: format!("http://localhost:9876/{}", id),
    });
    Ok((StatusCode::CREATED, body))
}

async fn redirect(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let url = state
        .get_url(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, url.parse().unwrap());
    Ok((headers, StatusCode::FOUND))
}

impl AppState {
    async fn new(db_url: &str) -> Result<Self> {
        let db = PgPool::connect(db_url).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS urls (
                id CHAR(6) PRIMARY KEY,
                url TEXT NOT NULL UNIQUE
            )",
        )
        .execute(&db)
        .await?;
        Ok(Self { db })
    }
    async fn shorten(&self, url: &str) -> Result<String> {
        let id = nanoid!(6);
        let ret: UrlRecord = sqlx::query_as(
            "INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT (url) DO UPDATE SET url=excluded.url RETURNING id",
        )
        .bind(&id)
        .bind(url)
        .fetch_one(&self.db)
        .await?;
        Ok(ret.id)
    }
    async fn get_url(&self, id: &str) -> Result<String> {
        let url: UrlRecord = sqlx::query_as("SELECT url FROM urls WHERE id = $1")
            .bind(id)
            .fetch_one(&self.db)
            .await?;
        Ok(url.url)
    }
}
