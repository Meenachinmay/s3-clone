use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub storage_path: String,
    pub server_addr: String,
    pub server_port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            storage_path: env::var("STORAGE_PATH").unwrap_or_else(|_| "./storage".to_string()),
            server_addr: env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("SERVER_PORT must be a valid number"),
        }
    }
}