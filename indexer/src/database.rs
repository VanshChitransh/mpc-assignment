use sqlx::PgPool;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct BalanceRecord {
    pub address: String,
    pub balance: i64,
    pub last_updated: DateTime<Utc>,
    pub slot: i64,
}

#[derive(Debug, Clone)]
pub struct TokenBalanceRecord {
    pub token_account: String,
    pub mint: String,
    pub amount: i64,
    pub slot: i64,
}

#[derive(Debug, Clone)]
pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get current SOL balance for an address
    pub async fn get_current_balance(&self, address: &str) -> Result<i64, sqlx::Error> {
        let balance = sqlx::query_scalar!(
            "SELECT sol_balance FROM user_wallets WHERE address = $1",
            address
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(balance.flatten().unwrap_or(0))
    }

    /// Get all addresses that need to be monitored
    pub async fn get_monitoring_addresses(&self) -> Result<Vec<String>, sqlx::Error> {
        let addresses = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT address 
            FROM user_wallets 
            WHERE address IS NOT NULL 
            AND is_active = true
            "#
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();

        Ok(addresses)
    }

    /// Get recently added addresses (for dynamic subscription updates)
    pub async fn get_recent_addresses(&self, minutes: i32) -> Result<Vec<String>, sqlx::Error> {
        let addresses = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT address 
            FROM user_wallets 
            WHERE address IS NOT NULL 
            AND is_active = true
            AND created_at > NOW() - INTERVAL '1 minute' * $1
            "#,
            minutes as f64
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();

        Ok(addresses)
    }

    /// Update indexer state (like last processed slot)
    pub async fn update_indexer_state(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO indexer_state (key, value, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (key) 
            DO UPDATE SET 
                value = EXCLUDED.value,
                updated_at = EXCLUDED.updated_at
            "#,
            key,
            value
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get indexer state value
    pub async fn get_indexer_state(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        let value = sqlx::query_scalar!(
            "SELECT value FROM indexer_state WHERE key = $1",
            key
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(value) // ✅ fixed
    }

    /// Get the last processed slot
    pub async fn get_last_processed_slot(&self) -> Result<i64, sqlx::Error> {
        let slot_str = self.get_indexer_state("last_processed_slot").await?;
        let slot = slot_str
            .unwrap_or_else(|| "0".to_string())
            .parse::<i64>()
            .unwrap_or(0);
        Ok(slot)
    }

    /// Update the last processed slot
    pub async fn update_last_processed_slot(&self, slot: i64) -> Result<(), sqlx::Error> {
        self.update_indexer_state("last_processed_slot", &slot.to_string()).await
    }

    /// Record a metric
    pub async fn record_metric(
        &self,
        metric_name: &str,
        metric_value: i64,
        tags: Option<Value>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO subscription_metrics (metric_name, metric_value, tags)
            VALUES ($1, $2, $3)
            "#,
            metric_name,
            metric_value,
            tags.unwrap_or_else(|| serde_json::json!({}))
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get balance history for an address
    pub async fn get_balance_history(
        &self,
        address: &str,
        limit: i64,
    ) -> Result<Vec<(DateTime<Utc>, i64, String)>, sqlx::Error> {
        let records = sqlx::query!(
            r#"
            SELECT created_at, new_balance, change_type
            FROM balance_changes 
            WHERE address = $1 
            ORDER BY created_at DESC 
            LIMIT $2
            "#,
            address,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let history = records
            .into_iter()
            .map(|row| {
                let created_at = row.created_at.unwrap_or_else(|| Utc::now());
                (created_at, row.new_balance, row.change_type)
            })
            .collect();

        Ok(history)
    }

    /// Get token balances for an address
    pub async fn get_token_balances(&self, owner: &str) -> Result<Vec<TokenBalanceRecord>, sqlx::Error> {
        let records = sqlx::query_as!(
            TokenBalanceRecord,
            r#"
            SELECT token_account, mint, amount, slot
            FROM token_balances
            WHERE token_account IN (
                SELECT address FROM user_wallets WHERE address LIKE $1 || '%'
            )
            AND amount > 0
            ORDER BY updated_at DESC
            "#,
            owner
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Insert or update user wallet
    pub async fn upsert_user_wallet(
        &self,
        user_id: &uuid::Uuid,
        address: &str,
        balance: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO user_wallets (user_id, address, sol_balance, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, true, NOW(), NOW())
            ON CONFLICT (address)
            DO UPDATE SET
                sol_balance = $3,
                updated_at = NOW(),
                is_active = true
            "#,
            user_id,
            address,
            balance
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a new user
    pub async fn create_user(&self, email: &str) -> Result<uuid::Uuid, sqlx::Error> {
        let user_id = sqlx::query_scalar!(
            r#"
            INSERT INTO users (id, email, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, NOW(), NOW())
            RETURNING id
            "#,
            email
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user_id)
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<uuid::Uuid>, sqlx::Error> {
        let user_id = sqlx::query_scalar!(
            "SELECT id FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user_id) // ✅ fixed
    }

    /// Get user wallets
    pub async fn get_user_wallets(&self, user_id: &uuid::Uuid) -> Result<Vec<String>, sqlx::Error> {
        let addresses = sqlx::query_scalar!(
            "SELECT address FROM user_wallets WHERE user_id = $1 AND is_active = true",
            user_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();

        Ok(addresses)
    }

    /// Update user total balance
    pub async fn update_user_total_balance(&self, user_id: &uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE users 
            SET sol_balance = (
                SELECT COALESCE(SUM(sol_balance), 0) 
                FROM user_wallets 
                WHERE user_id = $1 AND is_active = true
            ),
            updated_at = NOW()
            WHERE id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, sqlx::Error> {
        let user_count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let wallet_count = sqlx::query_scalar!("SELECT COUNT(*) FROM user_wallets WHERE is_active = true")
            .fetch_one(&self.pool)
            .await?;

        let balance_changes_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM balance_changes WHERE created_at > NOW() - INTERVAL '24 hours'"
        )
        .fetch_one(&self.pool)
        .await?;

        let total_balance = sqlx::query_scalar!(
            "SELECT CAST(COALESCE(SUM(sol_balance), 0) AS BIGINT) FROM users"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DatabaseStats {
            user_count: user_count.unwrap_or(0),
            wallet_count: wallet_count.unwrap_or(0),
            balance_changes_24h: balance_changes_count.unwrap_or(0),
            total_balance: total_balance.unwrap_or(0),
        })
    }

    /// Clean up old data
    pub async fn cleanup_old_data(&self, days: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            DELETE FROM balance_changes 
            WHERE created_at < NOW() - INTERVAL '1 day' * $1
            "#,
            days as f64
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar!("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        
        Ok(true)
    }
}

#[derive(Debug)]
pub struct DatabaseStats {
    pub user_count: i64,
    pub wallet_count: i64,
    pub balance_changes_24h: i64,
    pub total_balance: i64,
}
