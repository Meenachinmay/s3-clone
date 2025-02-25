pub mod jwt;
pub mod middleware;

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;
use self::jwt::JwtConfig;


#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    email: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    email: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    token: String,
    user_id: String,
    email: String,
}

pub async fn register(
    pool: web::Data<PgPool>,
    req: web::Json<RegisterRequest>,
) -> impl Responder {
    // Create a new user
    let user = User::new(req.email.clone());

    // Save the user to the database
    match user.create(&pool).await {
        Ok(_) => {
            // Return the API key to the client
            HttpResponse::Created().json(RegisterResponse {
                api_key: user.api_key,
            })
        }
        Err(e) => {
            // Check if the error is a unique constraint violation on email
            if let Some(db_error) = e.as_database_error() {
                if db_error.is_unique_violation() {
                    return HttpResponse::Conflict().json(serde_json::json!({
                        "error": "Email already registered"
                    }));
                }
            }

            // General error
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to register user"
            }))
        }
    }
}

pub async fn login(
    pool: web::Data<PgPool>,
    jwt_config: web::Data<JwtConfig>,
    req: web::Json<LoginRequest>,
) -> impl Responder {
    // Find user by email
    let user = match User::find_by_email(&pool, &req.email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid email"
            }));
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to verify credentials"
            }));
        }
    };

    // Generate JWT token
    match jwt_config.generate_token(user.id, &user.email) {
        Ok(token) => HttpResponse::Ok().json(LoginResponse {
            token,
            user_id: user.id.to_string(),
            email: user.email,
        }),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to generate authentication token"
        })),
    }
}