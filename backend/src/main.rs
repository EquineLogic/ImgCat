use backend::{AppData, config, routes};

use axum::{
    Router,
    http::{HeaderValue, Method},
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
        // Sharing
        .route(
            "/send_share_request",
            post(routes::sharing::send_share_request),
        )
        .route(
            "/cancel_share_request",
            post(routes::sharing::cancel_share_request),
        )
        .route(
            "/accept_share_request",
            post(routes::sharing::accept_share_request),
        )
        .route(
            "/decline_share_request",
            post(routes::sharing::decline_share_request),
        )
        .route(
            "/pending_requests",
            get(routes::sharing::list_pending_requests),
        )
        .route("/sent_requests", get(routes::sharing::list_sent_requests))
        .route(
            "/revoke_permission",
            post(routes::sharing::revoke_permission),
        )
        .route("/my_grants", get(routes::sharing::list_my_grants))
        .route("/shared_with_me", get(routes::sharing::list_shared_with_me))
        .route("/shared_folder", get(routes::sharing::list_shared_folder))
        .route("/shared_files", get(routes::sharing::list_shared_files))
        .route("/copy_shared_file", post(routes::sharing::copy_shared_file))
        // WebSocket
        .route("/ws", get(routes::ws::ws_handler))
        .route("/change_username", post(routes::auth::change_username))
        .route("/change_password", post(routes::auth::change_password))
        .route(
            "/trash_retention",
            get(routes::auth::get_trash_retention).post(routes::auth::set_trash_retention),
        )
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
