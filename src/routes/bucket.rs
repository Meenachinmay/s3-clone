use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::auth::{get_user_id_from_request};
use crate::models::Bucket;

#[derive(Debug, Deserialize)]
pub struct CreateBucketRequest {
    bucket_name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateBucketResponse {
    id: Uuid,
    name: String,
}

pub async fn create_bucket(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    bucket_req: web::Json<CreateBucketRequest>,
) -> impl Responder {
    // Get user ID from request extensions (set by middleware)
    let user_id = match get_user_id_from_request(&req) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Authentication required"
            }));
        }
    };

    // Validate bucket name
    let bucket_name = &bucket_req.bucket_name;
    if bucket_name.is_empty() || bucket_name.len() > 63 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Bucket name must be between 1 and 63 characters"
        }));
    }

    // Check if bucket already exists for this user
    match Bucket::find_by_name_and_user(&pool, bucket_name, user_id).await {
        Ok(Some(_)) => {
            return HttpResponse::Conflict().json(serde_json::json!({
                "error": "Bucket with this name already exists"
            }));
        }
        Ok(None) => {
            // Create new bucket
            let bucket = Bucket::new(bucket_name.clone(), user_id);

            // Save bucket to database
            match bucket.create(&pool).await {
                Ok(_) => {
                    HttpResponse::Created().json(CreateBucketResponse {
                        id: bucket.id,
                        name: bucket.name,
                    })
                }
                Err(_) => {
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to create bucket"
                    }))
                }
            }
        }
        Err(_) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to check existing buckets"
            }))
        }
    }
}