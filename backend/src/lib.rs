pub mod config;
pub mod notify;
pub mod ops;
pub mod routes;

use std::sync::Arc;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct AppData {
    pub pool: sqlx::PgPool,
    pub s3: aws_sdk_s3::Client,
    pub bucket: String,
    pub notify: Arc<notify::NotifyHub>,
}

impl AppData {
    pub async fn build() -> Result<Self, Error> {
        let cfg = &*config::CONFIG;

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(cfg.max_db_connections)
            .connect(&cfg.postgres_url)
            .await?;

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

        Ok(Self { pool, s3, bucket, notify: Arc::new(notify::NotifyHub::new()) })
    }
}
