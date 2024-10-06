use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json,
};
use http::{header::LOCATION, HeaderMap};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use thiserror::Error;

use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

//定义this error,
#[derive(Error, Debug)]
pub enum MyError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("retries limit error: {0}")]
    RetriesLimit(String),
    #[error("URL not found: {0}")]
    UrlNotFound(String),
}
impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            MyError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            MyError::RetriesLimit(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            MyError::UrlNotFound(e) => (StatusCode::NOT_FOUND, e.to_string()),
        };
        let body = Json(json!({
            "error":msg
        }));
        (status, body).into_response()
    }
}

struct DbState {
    db: sqlx::PgPool,
}

//sql返回类型定义，设置default 允许在数据返回时该值不存在
#[derive(Debug, FromRow)]
struct ShortUrl {
    #[sqlx(default)]
    id: String,
    #[sqlx(default)]
    url: String,
}

impl DbState {
    //创建数据库连接，创建表
    async fn new(db_url: &str) -> Result<Self, MyError> {
        let db = sqlx::PgPool::connect(db_url).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS short_urls (
                id CHAR(6) PRIMARY KEY,
                url TEXT NOT NULL UNIQUE
            )",
        )
        .execute(&db)
        .await?;

        Ok(Self { db })
    }
    //创建短链接
    async fn shorten(&self, url: &str) -> Result<String, MyError> {
        let mut retries = 3;
        //重试3次
        while retries > 0 {
            let id = nanoid!(6);
            let ret:Result<ShortUrl,sqlx::Error> = sqlx::query_as(
                "INSERT INTO short_urls (id,url) VALUES ($1,$2) on conflict (url) do update set url=excluded.url RETURNING id",
            )
            .bind(id)
            .bind(url)
            .fetch_one(&self.db)
            .await;
            match ret {
                Ok(ret) => return Ok(ret.id),
                //主键冲突，重试
                Err(sqlx::Error::Database(ref db_err)) if db_err.code().unwrap() == "23505" => {
                    retries -= 1;
                    info!("主键冲突，重试... 剩余重试次数: {}", retries);
                    continue;
                }
                //其他错误
                Err(e) => return Err(MyError::from(e)),
            }
        }
        //重试次数用尽
        Err(MyError::RetriesLimit("主键冲突且重试次数用尽".to_string()))
    }
    //获取短链接
    async fn get_url(&self, id: &str) -> Result<String, MyError> {
        let ret: Result<ShortUrl, sqlx::Error> =
            sqlx::query_as("SELECT url FROM short_urls WHERE id=$1")
                .bind(id)
                .fetch_one(&self.db)
                .await;
        match ret {
            Ok(ret) => {
                if ret.url.is_empty() {
                    return Err(MyError::UrlNotFound(format!("URL not found1: {}", id)));
                }
                Ok(ret.url)
            }
            Err(_e) => Err(MyError::UrlNotFound(id.to_string())),
        }
    }
}

//请求体，实现Deserialize，反序列化，Json中获取，所以需要反序列化
#[derive(Debug, Deserialize)]
struct UrlRequest {
    url: String,
}
//响应体，实现Serialize，序列化，返回Json，所以需要序列化
#[derive(Debug, Serialize)]
struct UrlResponse {
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().pretty().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let state =
        Arc::new(DbState::new("postgres://postgres:postgres@localhost:5432/shortener").await?);
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
    State(state): State<Arc<DbState>>,
    Json(payload): Json<UrlRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let id = state.shorten(&payload.url).await.map_err(|e| {
        warn!("{}", e);
        (StatusCode::UNPROCESSABLE_ENTITY, format!("{}", e))
    })?;
    let body = Json(UrlResponse {
        url: format!("http://localhost:9876/{}", id),
    });
    //返回201状态码，创建成功，返回是一个tuple，包含状态码和body，实现了IntoResponse
    Ok((StatusCode::CREATED, body))
}

async fn redirect(
    Path(id): Path<String>,
    State(state): State<Arc<DbState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let url = state.get_url(&id).await.map_err(|e| {
        warn!("{}", e);
        (StatusCode::NOT_FOUND, format!("{}", e))
    })?;
    //返回302状态码，重定向，返回是一个tuple，包含状态码和body，实现了IntoResponse
    //headermap，包含Location头，值为url
    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, url.parse().unwrap());
    Ok((headers, StatusCode::FOUND))
}
