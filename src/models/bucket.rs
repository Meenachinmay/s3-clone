use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl Bucket {
    pub fn new(name: String, user_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            user_id,
            created_at: Utc::now(),
        }
    }

    pub async fn create(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO buckets (id, name, user_id, created_at)
            VALUES ($1, $2, $3, $4)
            "#,
            self.id,
            self.name,
            self.user_id,
            self.created_at
        )
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn find_by_name_and_user(
        pool: &PgPool,
        name: &str,
        user_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let bucket = sqlx::query_as!(
            Bucket,
            r#"
            SELECT id, name, user_id, created_at
            FROM buckets
            WHERE name = $1 AND user_id = $2
            "#,
            name,
            user_id
        )
            .fetch_optional(pool)
            .await?;

        Ok(bucket)
    }
}