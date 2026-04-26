use backend::{AppData, config, routes};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, HeaderName, Method},
    routing::{get, post},
};
use reqwest::header;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let cfg: &config::Config = &*config::CONFIG;

    // setup logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let state = AppData::build()
        .await
        .expect("Failed to initialize app data");

    log::info!("Starting server on {}", cfg.bind_addr);

    let cors = CorsLayer::new()
        .allow_origin(
            cfg.allowed_origins
                .iter()
                .map(|o| HeaderValue::from_str(o).unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_methods([Method::POST, Method::GET])
        .allow_credentials(true)
        .allow_headers([header::CONTENT_TYPE, "X-Group".parse::<HeaderName>().unwrap(), header::AUTHORIZATION]);

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/check_auth", get(routes::check_auth))
        // WebSocket
        .route("/ws", get(routes::ws::ws_handler))
        .route("/op_anon", post(routes::op_anon))
        .route("/op_auth", post(routes::op_auth))
        .route("/upload_file_multipart", post(routes::upload_file_multipart))
        .with_state(state)
        .layer(cors)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)); // 100MB limit

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
