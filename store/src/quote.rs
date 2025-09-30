// store/src/quote.rs - Quote management module

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuoteError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Quote not found")]
    QuoteNotFound,
    #[error("Quote expired")]
    QuoteExpired,
    #[error("Quote already used")]
    QuoteAlreadyUsed,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Quote {
    pub id: Uuid,
    pub user_id: Uuid,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: i64,
    pub out_amount: i64,
    
    pub quote_data: Value,
    pub used: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl super::Store {
    /// Store a new quote
    pub async fn store_quote(
        &self,
        user_id: &Uuid,
        input_mint: &str,
        output_mint: &str,
        in_amount: i64,
        out_amount: i64,
        quote_data: Value,
        expiry_seconds: i64,
    ) -> Result<Quote, QuoteError> {
        let expires_at = Utc::now() + Duration::seconds(expiry_seconds);
        
        let quote = sqlx::query_as!(
            Quote,
            r#"
            INSERT INTO quotes 
            (user_id, input_mint, output_mint, in_amount, out_amount, 
             quote_data, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            RETURNING id, user_id, input_mint, output_mint, in_amount, 
                     out_amount, quote_data, used, 
                     expires_at, created_at, updated_at
            "#,
            user_id,
            input_mint,
            output_mint,
            in_amount,
            out_amount,
            quote_data,
            expires_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(quote)
    }

    /// Get a valid quote by ID
    pub async fn get_valid_quote(&self, quote_id: &Uuid, user_id: &Uuid) -> Result<Quote, QuoteError> {
        let quote = sqlx::query_as!(
            Quote,
            r#"
            SELECT id, user_id, input_mint, output_mint, in_amount, 
                   out_amount, quote_data, used, 
                   expires_at, created_at, updated_at
            FROM quotes
            WHERE id = $1 AND user_id = $2
            "#,
            quote_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?
        .ok_or(QuoteError::QuoteNotFound)?;

        // Check if quote is expired
        if Utc::now() > quote.expires_at {
            return Err(QuoteError::QuoteExpired);
        }

        // Check if quote is already used
        if quote.used {
            return Err(QuoteError::QuoteAlreadyUsed);
        }

        Ok(quote)
    }

    /// Mark a quote as used
    pub async fn mark_quote_used(&self, quote_id: &Uuid) -> Result<(), QuoteError> {
        let result = sqlx::query!(
            "UPDATE quotes SET used = true, updated_at = NOW() WHERE id = $1",
            quote_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QuoteError::QuoteNotFound);
        }

        Ok(())
    }

    /// Clean up expired quotes
    pub async fn cleanup_expired_quotes(&self) -> Result<u64, QuoteError> {
        let result = sqlx::query!(
            "DELETE FROM quotes WHERE expires_at < NOW() AND used = false"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected())
    }
}

#[derive(Debug)]
pub struct QuoteStats {
    pub total_quotes: i64,
    pub used_quotes: i64,
    pub expired_quotes: i64,
    pub active_quotes: i64,
}

impl super::Store {
    /// Get quote statistics
    pub async fn get_quote_stats(&self) -> Result<QuoteStats, QuoteError> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_quotes,
                COUNT(*) FILTER (WHERE used = true) as used_quotes,
                COUNT(*) FILTER (WHERE expires_at < NOW()) as expired_quotes,
                COUNT(*) FILTER (WHERE used = false AND expires_at > NOW()) as active_quotes
            FROM quotes
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(QuoteStats {
            total_quotes: stats.total_quotes.unwrap_or(0),
            used_quotes: stats.used_quotes.unwrap_or(0),
            expired_quotes: stats.expired_quotes.unwrap_or(0),
            active_quotes: stats.active_quotes.unwrap_or(0),
        })
    }

    /// Clean up old used quotes
    pub async fn cleanup_old_used_quotes(&self, days_old: i32) -> Result<u64, QuoteError> {
        let result = sqlx::query!(
            "DELETE FROM quotes WHERE used = true AND created_at < NOW() - INTERVAL '1 day' * $1",
            days_old as f64
        )
        .execute(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected())
    }
}
