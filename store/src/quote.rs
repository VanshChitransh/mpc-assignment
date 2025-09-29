use crate::{Store, Quote, QuoteError};
use serde_json::Value;
use uuid::Uuid;

impl Store {
    /// Store a new quote from Jupiter API
    pub async fn store_quote(
        &self,
        user_id: &Uuid,
        input_mint: &str,
        output_mint: &str,
        in_amount: i64,
        out_amount: i64,
        quote_data: Value,
        expires_in_seconds: i64,
    ) -> Result<Quote, QuoteError> {
        let quote = Quote::new(
            user_id.clone(),
            input_mint.to_string(),
            output_mint.to_string(),
            in_amount,
            out_amount,
            quote_data,
            expires_in_seconds,
        );

        sqlx::query!(
            r#"
            INSERT INTO quotes (id, user_id, input_mint, output_mint, in_amount, out_amount, quote_data, expires_at, created_at, used, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            quote.id,
            quote.user_id,
            quote.input_mint,
            quote.output_mint,
            quote.in_amount,
            quote.out_amount,
            quote.quote_data,
            quote.expires_at,
            quote.created_at,
            quote.used,
            quote.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(quote)
    }

    /// Retrieve a quote by ID
    pub async fn get_quote(&self, quote_id: &Uuid) -> Result<Quote, QuoteError> {
        let quote = sqlx::query_as!(
            Quote,
            "SELECT * FROM quotes WHERE id = $1",
            quote_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        quote.ok_or(QuoteError::QuoteNotFound)
    }

    /// Retrieve and validate a quote for use (checks expiration and usage)
    pub async fn get_valid_quote(&self, quote_id: &Uuid, user_id: &Uuid) -> Result<Quote, QuoteError> {
        let quote = sqlx::query_as!(
            Quote,
            "SELECT * FROM quotes WHERE id = $1 AND user_id = $2",
            quote_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?
        .ok_or(QuoteError::QuoteNotFound)?;

        // Check if quote is expired
        if quote.is_expired() {
            return Err(QuoteError::QuoteExpired);
        }

        // Check if quote has already been used
        if quote.used {
            return Err(QuoteError::QuoteAlreadyUsed);
        }

        Ok(quote)
    }

    /// Mark a quote as used
    pub async fn mark_quote_used(&self, quote_id: &Uuid) -> Result<(), QuoteError> {
        let result = sqlx::query!(
            "UPDATE quotes SET used = true, updated_at = NOW() WHERE id = $1 AND used = false",
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

    /// Get all quotes for a user (for debugging/admin purposes)
    pub async fn get_user_quotes(&self, user_id: &Uuid, limit: Option<i64>) -> Result<Vec<Quote>, QuoteError> {
        let limit = limit.unwrap_or(50);

        let quotes = sqlx::query_as!(
            Quote,
            "SELECT * FROM quotes WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
            user_id,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(quotes)
    }

    /// Clean up expired quotes (should be run periodically)
    pub async fn cleanup_expired_quotes(&self) -> Result<u64, QuoteError> {
        let result = sqlx::query!(
            "DELETE FROM quotes WHERE expires_at < NOW()"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Clean up old used quotes (older than specified days)
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

    /// Get quote statistics for monitoring
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
}

#[derive(Debug)]
pub struct QuoteStats {
    pub total_quotes: i64,
    pub used_quotes: i64,
    pub expired_quotes: i64,
    pub active_quotes: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use serde_json::json;

    #[tokio::test]
    async fn test_quote_operations() {
        if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
            let pool = Store::new_pool(&database_url).await.unwrap();
            let store = Store::new(pool);

            let user_id = Uuid::new_v4();
            let quote_data = json!({
                "inputMint": "So11111111111111111111111111111111111111112",
                "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "swapMode": "ExactIn"
            });

            // Test storing a quote
            let quote = store.store_quote(
                &user_id,
                "So11111111111111111111111111111111111111112",
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                1000000000,
                100000000,
                quote_data,
                300 // 5 minutes
            ).await.unwrap();

            // Test retrieving the quote
            let retrieved = store.get_quote(&quote.id).await.unwrap();
            assert_eq!(retrieved.id, quote.id);
            assert_eq!(retrieved.user_id, user_id);

            // Test getting valid quote
            let valid_quote = store.get_valid_quote(&quote.id, &user_id).await.unwrap();
            assert!(!valid_quote.used);
            assert!(!valid_quote.is_expired());

            // Test marking quote as used
            store.mark_quote_used(&quote.id).await.unwrap();
            let used_result = store.get_valid_quote(&quote.id, &user_id).await;
            assert!(matches!(used_result, Err(QuoteError::QuoteAlreadyUsed)));
        }
    }
}