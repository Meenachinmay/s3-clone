mod authentication;
mod config;
mod db;
mod middleware;
mod models;
mod handlers;
mod storage;

use actix_web::{web, App, HttpServer};
use actix_web::middleware::Logger; // Import Logger specifically
use std::sync::Arc;
use actix_cors::Cors;
use crate::config::Config;
use crate::db::postgres::init_pool;
use crate::middleware::auth::ApiKeyMiddleware;
use authentication::middleware::AuthMiddleware;
use crate::handlers::{bucket, file};
use crate::storage::local::LocalStorage;
use crate::storage::Storage;
use env_logger::Env;
use log::{error, info};
use crate::authentication::jwt::JwtConfig;
use crate::authentication::middleware::AuthMiddlewareService;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Load configuration
    let config = Config::from_env();
    info!("Configuration loaded: {:?}", config);

    // Initialize database pool
    let pool = match init_pool(&config.database_url).await {
        Ok(pool) => {
            info!("Database connection established");
            pool
        }
        Err(err) => {
            error!("Failed to connect to database: {:?}", err);
            panic!("Failed to connect to database: {:?}", err);
        }
    };

    // Initialize storage
    let storage = match LocalStorage::new(&config.storage_path) {
        Ok(storage) => {
            info!("Local storage initialized at: {}", config.storage_path);
            Arc::new(storage) as Arc<dyn Storage + Send + Sync>
        }
        Err(err) => {
            error!("Failed to initialize storage: {:?}", err);
            panic!("Failed to initialize storage: {:?}", err);
        }
    };

    // Initialize JWT config
    // In production, get this from environment variables
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secretkey".to_string());
    let jwt_config = JwtConfig::new(jwt_secret, 24 * 60 * 60); // 24 hours expiration

    // Start HTTP server
    info!(
        "Starting server at {}:{}",
        config.server_addr, config.server_port
    );

    HttpServer::new(move || {
        // Configure CORS middleware
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(Logger::new("%r %s %{User-Agent}i %D ms"))  // Add detailed logging
            .wrap(cors)  // Add CORS middleware
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::from(storage.clone()))
            .app_data(web::Data::new(jwt_config.clone()))
            .service(
                web::resource("/register")
                    .route(web::post().to(authentication::register))
            )
            .service (
                web::resource("/login")
                    .route(web::post().to(authentication::login))
            )
            .service (
                web::resource("/buckets")
                    .wrap(AuthMiddleware {
                        pool: pool.clone(),
                        jwt_config: jwt_config.clone()
                    })
                    .route(web::get().to(bucket::list_buckets))
            )
            .service (
                web::resource("/files")
                    .wrap(AuthMiddleware {
                        pool: pool.clone(),
                        jwt_config: jwt_config.clone()
                    })
                    .route(web::get().to(file::list_files))
            )
            .service(
                web::resource("/create-bucket")
                    .wrap( AuthMiddleware {
                        pool: pool.clone(),
                        jwt_config: jwt_config.clone(),
                    })
                    .route(web::post().to(bucket::create_bucket))
            )
            .service(
                web::resource("/upload-file")
                    .wrap( AuthMiddleware {
                        pool: pool.clone(),
                        jwt_config: jwt_config.clone(),
                    })
                    .route(web::post().to(file::upload_file))
            )
            .service(
                web::resource("/get-file")
                    .wrap(AuthMiddleware {
                        pool: pool.clone(),
                        jwt_config: jwt_config.clone(),
                    })
                    .route(web::get().to(file::get_file_info))
            )
    })
        .bind((config.server_addr, config.server_port))?
        .run()
        .await
}