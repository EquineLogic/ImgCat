mod config;
mod models;
mod routes;

use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{get, post},
};
use reqwest::header;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct AppData {
    pool: sqlx::PgPool,
    s3: aws_sdk_s3::Client,
    bucket: String,
}

#[tokio::main]
async fn main() {
    // setup logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cfg = &*config::CONFIG;

    let pool = PgPoolOptions::new()
        .max_connections(cfg.max_db_connections)
        .connect(&cfg.postgres_url)
        .await
        .expect("Could not initialize connection");

    let s3_config = aws_sdk_s3::Config::builder()
        .endpoint_url(cfg.object_storage.endpoint.as_deref().unwrap())
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            cfg.object_storage.access_key.as_deref().unwrap(),
            cfg.object_storage.secret_key.as_deref().unwrap(),
            None,
            None,
            "Static",
        ))
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .force_path_style(true)
        .behavior_version_latest()
        .build();

    let s3 = aws_sdk_s3::Client::from_conf(s3_config);

    let bucket = cfg.object_storage.bucket.clone().unwrap();

    log::info!("Starting server on http://localhost:3000");

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
        .route("/signout", post(routes::auth::sign_out))
        .route("/create_folder", post(routes::filesystem::create_folder))
        .route("/list_folders", get(routes::filesystem::list_folders))
        .route("/delete_folder", post(routes::filesystem::delete_folder))
        .route("/rename_folder", post(routes::filesystem::rename_folder))
        .route("/upload_file", post(routes::filesystem::upload_file))
        .route("/list_files", get(routes::filesystem::list_files))
        .route("/files/{id}", get(routes::filesystem::get_file))
        .route("/rename_file", post(routes::filesystem::rename_file))
        .route("/delete_file", post(routes::filesystem::delete_file))
        .route("/reorder", post(routes::filesystem::reorder))
        .route("/move", post(routes::filesystem::move_entry))
        .route("/list_trash", get(routes::filesystem::list_trash))
        .route("/restore", post(routes::filesystem::restore_entry))
        .route(
            "/delete_trash_entry",
            post(routes::filesystem::delete_trash_entry),
        )
        .with_state(AppData { pool, s3, bucket })
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
