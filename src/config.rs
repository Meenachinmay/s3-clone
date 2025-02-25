use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub storage_path: String,
    pub server_addr: String,
    pub server_port: u16,
    pub jwt_secret: String,
    pub jwt_expiration: i64, // In seconds
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            storage_path: env::var("STORAGE_PATH").unwrap_or_else(|_| "./storage".to_string()),
            server_addr: env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("SERVER_PORT must be a valid number"),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| "secretkey".to_string()),
            jwt_expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string()) // 24 hours in seconds
                .parse()
                .expect("JWT_EXPIRATION must be a valid number"),
        }
    }
}