pub mod config;
pub mod notify;
pub mod ops;
pub mod routes;
pub mod migrations;

use std::sync::Arc;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct AppData {
    pub pool: sqlx::PgPool,
    pub cdn_s3: aws_sdk_s3::Client,
    pub local_s3: aws_sdk_s3::Client,
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

        let local_s3_config = aws_sdk_s3::Config::builder()
            .endpoint_url(&cfg.object_storage.local_endpoint)
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                &cfg.object_storage.access_key,
                &cfg.object_storage.secret_key,
                None,
                None,
                "Static",
            ))
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .force_path_style(true)
            .behavior_version_latest()
            .build();

        let cdn_s3_config = aws_sdk_s3::Config::builder()
            .endpoint_url(&cfg.object_storage.cdn_endpoint)
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                &cfg.object_storage.access_key,
                &cfg.object_storage.secret_key,
                None,
                None,
                "Static",
            ))
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .force_path_style(true)
            .behavior_version_latest()
            .build();

        let local_s3 = aws_sdk_s3::Client::from_conf(local_s3_config);
        let cdn_s3 = aws_sdk_s3::Client::from_conf(cdn_s3_config);
        let bucket = cfg.object_storage.bucket.clone();

        Ok(Self { pool, local_s3, cdn_s3, bucket, notify: Arc::new(notify::NotifyHub::new()) })
    }
}
