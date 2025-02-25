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

#[derive(Debug, Serialize)]
pub struct BucketListResponse {
    buckets: Vec<BucketInfo>,
}

#[derive(Debug, Serialize)]
pub struct BucketInfo {
    id: Uuid,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
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

pub async fn list_buckets(
    req: HttpRequest,
    pool: web::Data<PgPool>,
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

    // Find all buckets for this user
    match Bucket::find_by_user_id(&pool, user_id).await {
        Ok(buckets) => {
            // Convert buckets to response format
            let bucket_infos = buckets.into_iter().map(|bucket| {
                BucketInfo {
                    id: bucket.id,
                    name: bucket.name,
                    created_at: bucket.created_at,
                }
            }).collect();

            HttpResponse::Ok().json(BucketListResponse {
                buckets: bucket_infos,
            })
        }
        Err(e) => {
            eprintln!("Error fetching buckets: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch buckets"
            }))
        }
    }
}