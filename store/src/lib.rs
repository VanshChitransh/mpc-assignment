use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{info, error};

#[derive(Debug, Error)]
pub enum UserError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Email already exists: {0}")]
    EmailAlreadyExists(String),
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub public_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct UserStore<'a> {
    pool: &'a PgPool,
}

impl<'a> UserStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
    
    /// Create a new user
    pub async fn create_user(&self, email: &str, password_hash: &str) -> Result<User, UserError> {
        // Check if email already exists
        let existing = sqlx::query!(
            "SELECT id FROM users WHERE email = $1",
            email.to_lowercase()
        )
        .fetch_optional(self.pool)
        .await?;
        
        if existing.is_some() {
            return Err(UserError::EmailAlreadyExists(email.to_string()));
        }
        
        // Create user
        let now = Utc::now();
        let id = Uuid::new_v4();
        
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, email, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, email, password_hash, public_key, created_at, updated_at
            "#,
            id,
            email.to_lowercase(),
            password_hash,
            now,
            now
        )
        .fetch_one(self.pool)
        .await?;
        
        info!("Created user: {} ({})", email, id);
        Ok(user)
    }
    
    /// Get a user by ID
    pub async fn get_user_by_id(&self, user_id: &Uuid) -> Result<User, UserError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password_hash, public_key, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(self.pool)
        .await?;
        
        match user {
            Some(user) => Ok(user),
            None => Err(UserError::UserNotFound(user_id.to_string())),
        }
    }
    
    /// Get a user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<User, UserError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password_hash, public_key, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email.to_lowercase()
        )
        .fetch_optional(self.pool)
        .await?;
        
        match user {
            Some(user) => Ok(user),
            None => Err(UserError::UserNotFound(email.to_string())),
        }
    }
    
    /// Update user's public key
    pub async fn update_user_public_key(&self, user_id: &Uuid, public_key: &str) -> Result<User, UserError> {
        let now = Utc::now();
        
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET public_key = $1, updated_at = $2
            WHERE id = $3
            RETURNING id, email, password_hash, public_key, created_at, updated_at
            "#,
            public_key,
            now,
            user_id
        )
        .fetch_optional(self.pool)
        .await?;
        
        match user {
            Some(user) => {
                info!("Updated public key for user: {}", user_id);
                Ok(user)
            },
            None => Err(UserError::UserNotFound(user_id.to_string())),
        }
    }
    
    /// Check if a user exists
    pub async fn user_exists(&self, email: &str) -> Result<bool, UserError> {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1) as exists",
            email.to_lowercase()
        )
        .fetch_one(self.pool)
        .await?;
        
        Ok(result.exists.unwrap_or(false))
    }
}