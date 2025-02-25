use chrono::{DateTime, Utc};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub api_key: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String) -> Self {
        // Generate a random API key
        let api_key = Self::generate_api_key();

        Self {
            id: Uuid::new_v4(),
            email,
            api_key,
            created_at: Utc::now(),
        }
    }

    fn generate_api_key() -> String {
        let random_bytes: [u8; 32] = thread_rng().gen();
        let mut hasher = Sha256::new();
        hasher.update(&random_bytes);
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub async fn create(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, api_key, created_at)
            VALUES ($1, $2, $3, $4)
            "#,
            self.id,
            self.email,
            self.api_key,
            self.created_at
        )
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn find_by_api_key(pool: &PgPool, api_key: &str) -> Result<Option<Self>, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, api_key, created_at
            FROM users
            WHERE api_key = $1
            "#,
            api_key
        )
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, api_key, created_at
        FROM users
        WHERE email = $1
        "#,
        email
    )
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }
}