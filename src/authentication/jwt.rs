use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, errors::Error as JwtError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;

// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // Subject (user ID)
    pub email: String,     // User email
    pub exp: i64,          // Expiration time
    pub iat: i64,          // Issued at time
}

// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration: i64, // In seconds
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "secretkey".to_string(), // In production, use environment variable
            expiration: 24 * 60 * 60, // 24 hours in seconds
        }
    }
}

impl JwtConfig {
    pub fn new(secret: String, expiration: i64) -> Self {
        Self { secret, expiration }
    }

    // Generate a JWT token for a user
    pub fn generate_token(&self, user_id: Uuid, email: &str) -> Result<String, JwtError> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.expiration);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    // Validate a JWT token and extract claims
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )?;

        Ok(token_data.claims)
    }
}