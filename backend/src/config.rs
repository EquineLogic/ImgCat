use serde::{Deserialize, Serialize};
use std::fs::File;
use std::net::SocketAddr;
use std::sync::LazyLock;

/// Global config object
pub static CONFIG: LazyLock<Config> =
    LazyLock::new(|| Config::load().expect("Failed to load config"));

/// For S3->Seaweed
#[derive(Serialize, Deserialize)]
pub struct ObjectStorage {
    pub cdn_endpoint: String,
    pub local_endpoint: String,
    pub secure: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub max_db_connections: u32,
    pub postgres_url: String,
    pub object_storage: ObjectStorage,
    pub bind_addr: SocketAddr,
    pub allowed_origins: Vec<String>,
    pub allow_register: bool,

    #[serde(skip)]
    /// Setup by load() for statistics
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl Config {
    pub fn load() -> Result<Self, crate::Error> {
        // Open config.yaml from parent directory
        let file = File::open("config.yaml");

        match file {
            Ok(file) => {
                // Parse config.yaml
                let mut cfg: Config = serde_yaml::from_reader(file)?;

                cfg.start_time = chrono::Utc::now();

                // Return config
                Ok(cfg)
            }
            Err(e) => {
                // Print error
                println!("config.yaml could not be loaded: {}", e);

                // Exit
                std::process::exit(1);
            }
        }
    }
}
