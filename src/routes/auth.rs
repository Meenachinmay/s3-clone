use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::models::User;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    email: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    api_key: String,
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