use sqlx::PgPool;
use uuid::Uuid;
use thiserror::Error;
use serde_json::Value;
use chrono::{DateTime, Utc};
use tracing::{info, error};

#[derive(Debug, Error)]
pub enum QuoteError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Quote not found: {0}")]
    QuoteNotFound(Uuid),
    
    #[error("Quote expired")]
    QuoteExpired,
    
    #[error("Quote already used")]
    QuoteAlreadyUsed,
    
    #[error("Unauthorized: user {0} does not own quote {1}")]
    Unauthorized(Uuid, Uuid),
}

#[derive(Debug, Clone)]
pub struct Quote {
    pub id: Uuid,
    pub user_id: Uuid,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: i64,
    pub out_amount: i64,
    pub quote_data: Value,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct QuoteStore<'a> {
    pool: &'a PgPool,
}

impl<'a> QuoteStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
    
    /// Create a new quote
    pub async fn create_quote(
        &self,
        user_id: &Uuid,
        input_mint: &str,
        output_mint: &str,
        in_amount: i64,
        out_amount: i64,
        quote_data: Value,
        expires_at: DateTime<Utc>,
    ) -> Result<Quote, QuoteError> {
        let quote = sqlx::query_as!(
            Quote,
            r#"
            INSERT INTO quotes (
                user_id, input_mint, output_mint, 
                in_amount, out_amount, quote_data, 
                expires_at, used
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, false)
            RETURNING 
                id, user_id, input_mint, output_mint, 
                in_amount, out_amount, quote_data as "quote_data!: Value", 
                expires_at, used, created_at, updated_at
            "#,
            user_id,
            input_mint,
            output_mint,
            in_amount,
            out_amount,
            quote_data as _,
            expires_at,
        )
        .fetch_one(self.pool)
        .await?;
        
        Ok(quote)
    }
    
    /// Get a quote by ID
    pub async fn get_quote(&self, id: &Uuid) -> Result<Quote, QuoteError> {
        let quote = sqlx::query_as!(
            Quote,
            r#"
            SELECT 
                id, user_id, input_mint, output_mint, 
                in_amount, out_amount, quote_data as "quote_data!: Value", 
                expires_at, used, created_at, updated_at
            FROM quotes
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool)
        .await?;
        
        match quote {
            Some(quote) => Ok(quote),
            None => Err(QuoteError::QuoteNotFound(*id)),
        }
    }
    
    /// Get a valid quote (not expired, not used) for a specific user
    pub async fn get_valid_quote(&self, id: &Uuid, user_id: &Uuid) -> Result<Quote, QuoteError> {
        let now = Utc::now();
        
        let quote = sqlx::query_as!(
            Quote,
            r#"
            SELECT 
                id, user_id, input_mint, output_mint, 
                in_amount, out_amount, quote_data as "quote_data!: Value", 
                expires_at, used, created_at, updated_at
            FROM quotes
            WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id
        )
        .fetch_optional(self.pool)
        .await?;
        
        let quote = match quote {
            Some(quote) => quote,
            None => return Err(QuoteError::QuoteNotFound(*id)),
        };
        
        // Check if user is authorized
        if quote.user_id != *user_id {
            return Err(QuoteError::Unauthorized(*user_id, *id));
        }
        
        // Check if quote is expired
        if quote.expires_at < now {
            return Err(QuoteError::QuoteExpired);
        }
        
        // Check if quote is already used
        if quote.used {
            return Err(QuoteError::QuoteAlreadyUsed);
        }
        
        Ok(quote)
    }
    
    /// Mark a quote as used
    pub async fn mark_quote_used(&self, id: &Uuid) -> Result<Quote, QuoteError> {
        let quote = sqlx::query_as!(
            Quote,
            r#"
            UPDATE quotes
            SET used = true, updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, user_id, input_mint, output_mint, 
                in_amount, out_amount, quote_data as "quote_data!: Value", 
                expires_at, used, created_at, updated_at
            "#,
            id
        )
        .fetch_optional(self.pool)
        .await?;
        
        match quote {
            Some(quote) => Ok(quote),
            None => Err(QuoteError::QuoteNotFound(*id)),
        }
    }
    
    /// Clean up expired quotes
    pub async fn cleanup_expired_quotes(&self) -> Result<u64, QuoteError> {
        let now = Utc::now();
        
        let result = sqlx::query!(
            r#"
            DELETE FROM quotes
            WHERE expires_at < $1
            "#,
            now
        )
        .execute(self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
}