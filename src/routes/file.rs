use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::io::Write;
use log::{error, info};
use uuid::Uuid;

use crate::models::{Bucket, File};
use crate::storage::Storage;
use crate::middleware::auth::get_user_id_from_request;

#[derive(Debug, Deserialize)]
pub struct UploadFileQuery {
    bucket_name: String,
}

#[derive(Debug, Serialize)]
pub struct FileInfoResponse {
    id: Uuid,
    filename: String,
    content_type: Option<String>,
    size: i64,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn upload_file(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    storage: web::Data<dyn Storage + Send + Sync>,
    query: web::Query<UploadFileQuery>,
    mut payload: Multipart,
) -> impl Responder {
    // Get user ID from request extensions (set by middleware)
    let user_id = match get_user_id_from_request(&req) {
        Some(id) => id,
        None => {
            error!("Authentication failed: No user ID in request");
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Authentication required"
            }));
        }
    };

    info!("Processing file upload for user: {}, bucket: {}", user_id, query.bucket_name);

    // Find bucket by name and user
    let bucket = match Bucket::find_by_name_and_user(&pool, &query.bucket_name, user_id).await {
        Ok(Some(bucket)) => bucket,
        Ok(None) => {
            error!("Bucket not found: {}", query.bucket_name);
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Bucket not found"
            }));
        }
        Err(e) => {
            error!("Database error when checking bucket: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to check bucket"
            }));
        }
    };

    // Process the multipart upload
    let mut field_count = 0;

    while let Ok(Some(mut field)) = payload.try_next().await {
        field_count += 1;
        info!("Processing field: {:?}", field.name());

        let content_disposition = field.content_disposition();

        // Get filename from content disposition
        let filename = match content_disposition.get_filename() {
            Some(name) => name.to_string(),
            None => {
                error!("No filename provided in the upload");
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Filename is required"
                }));
            }
        };

        info!("Uploading file: {}", filename);

        // Get content-type
        let content_type = field
            .content_type()
            .map(|ct| ct.to_string());

        // Read file content
        let mut file_content = Vec::new();

        while let Some(chunk) = field.next().await {
            let data = match chunk {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to read chunk: {:?}", e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to read upload data"
                    }));
                }
            };

            // Write chunk to buffer
            if let Err(e) = file_content.write_all(&data) {
                error!("Failed to write chunk to buffer: {:?}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to process upload data"
                }));
            }
        }

        info!("File content read, size: {} bytes", file_content.len());

        // Generate a unique file ID
        let file_id = Uuid::new_v4();

        // Save file to storage
        info!("Saving file to storage...");
        let storage_path = match storage
            .save_file(&bucket.name, file_id, &filename, &file_content)
            .await
        {
            Ok(path) => {
                info!("File saved to: {}", path);
                path
            },
            Err(e) => {
                error!("Failed to save file to storage: {:?}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to save file to storage"
                }));
            }
        };

        // Create file record in database
        let file = File::new(
            filename,
            content_type,
            file_content.len() as i64,
            bucket.id,
            storage_path,
        );

        info!("Creating database record for file: {}", file.id);

        match file.create(&pool).await {
            Ok(_) => {
                info!("File uploaded successfully: {}", file.id);
                return HttpResponse::Created().json(FileInfoResponse {
                    id: file.id,
                    filename: file.filename,
                    content_type: file.content_type,
                    size: file.size,
                    created_at: file.created_at,
                });
            }
            Err(e) => {
                error!("Failed to save file metadata to DB: {:?}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to save file metadata"
                }));
            }
        }
    }

    // If we got here, no fields were processed
    error!("No file fields found in the upload request. Field count: {}", field_count);
    HttpResponse::BadRequest().json(serde_json::json!({
        "error": "No file uploaded"
    }))
}

#[derive(Debug, Deserialize)]
pub struct GetFileQuery {
    bucket_name: String,
    filename: String,
}

pub async fn get_file_info(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    query: web::Query<GetFileQuery>,
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

    // Find bucket by name and user
    let bucket = match Bucket::find_by_name_and_user(&pool, &query.bucket_name, user_id).await {
        Ok(Some(bucket)) => bucket,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Bucket not found"
            }));
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to check bucket"
            }));
        }
    };

    // Find file by filename and bucket
    match File::find_by_filename_and_bucket(&pool, &query.filename, bucket.id).await {
        Ok(Some(file)) => {
            HttpResponse::Ok().json(FileInfoResponse {
                id: file.id,
                filename: file.filename,
                content_type: file.content_type,
                size: file.size,
                created_at: file.created_at,
            })
        }
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "File not found"
            }))
        }
        Err(_) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch file info"
            }))
        }
    }
}