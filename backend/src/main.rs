mod config;
mod models;
mod routes;

use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{get, post},
};
use log::info;
use reqwest::header;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct AppData {
    pool: sqlx::PgPool,
    reqwest: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // setup logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let reqwest = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .expect("Could not initialize reqwest client");

    let cfg = &*config::CONFIG;

    let pool = PgPoolOptions::new()
        .max_connections(cfg.max_db_connections)
        .connect(&cfg.postgres_url)
        .await
        .expect("Could not initialize connection");

    info!("Starting server on http://localhost:3000");

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::POST, Method::GET])
        .allow_credentials(true)
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/register", post(routes::auth::register))
        .route("/signin", post(routes::auth::sign_in))
        .route("/check_auth", get(routes::auth::check_auth))
        .with_state(AppData { pool, reqwest })
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
