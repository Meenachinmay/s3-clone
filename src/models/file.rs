use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct File {
    pub id: Uuid,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub bucket_id: Uuid,
    pub storage_path: String,
    pub created_at: DateTime<Utc>,
}

impl File {
    pub fn new(
        filename: String,
        content_type: Option<String>,
        size: i64,
        bucket_id: Uuid,
        storage_path: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            filename,
            content_type,
            size,
            bucket_id,
            storage_path,
            created_at: Utc::now(),
        }
    }

    pub async fn create(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO files (id, filename, content_type, size, bucket_id, storage_path, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            self.id,
            self.filename,
            self.content_type,
            self.size,
            self.bucket_id,
            self.storage_path,
            self.created_at
        )
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn find_by_filename_and_bucket(
        pool: &PgPool,
        filename: &str,
        bucket_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let file = sqlx::query_as!(
            File,
            r#"
            SELECT id, filename, content_type, size, bucket_id, storage_path, created_at
            FROM files
            WHERE filename = $1 AND bucket_id = $2
            "#,
            filename,
            bucket_id
        )
            .fetch_optional(pool)
            .await?;

        Ok(file)
    }

    pub async fn find_by_bucket_id(
        pool: &PgPool,
        bucket_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let files = sqlx::query_as!(
        File,
        r#"
        SELECT id, filename, content_type, size, bucket_id, storage_path, created_at
        FROM files
        WHERE bucket_id = $1
        ORDER BY created_at DESC
        "#,
        bucket_id
    )
            .fetch_all(pool)
            .await?;

        Ok(files)
    }
}