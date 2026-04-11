use backend::{AppData, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let app = AppData::build()
        .await
        .expect("Failed to initialize app data");

    backend::migrations::apply_migrations(app.pool.clone()).await?;
    Ok(())
}