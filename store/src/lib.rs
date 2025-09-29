pub mod user;
pub mod models;
pub mod balance;
pub mod quote;

use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use sqlx::types::Uuid;
use crate::user::UserStats;
use crate::quote::QuoteStats;
// use uuid::Uuid;

pub use models::*;

pub struct Store {
    pub pool: PgPool,
}

impl Store {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn new_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
        PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("../migrations").run(&self.pool).await
    }

    pub async fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        
        let pool = Self::new_pool(&database_url).await?;
        let store = Self::new(pool);

        store.migrate().await?;
        
        Ok(store)
    }

    pub async fn from_url(database_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = Self::new_pool(database_url).await?;
        let store = Self::new(pool);

        store.migrate().await?;
        
        Ok(store)
    }

    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn detailed_health_check(&self) -> Result<HealthStatus, sqlx::Error> {
        use std::time::Instant;
        
        let start = Instant::now();

        let connected = sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .is_ok();
        
        let response_time_ms = start.elapsed().as_millis() as u64;

        let tables_exist = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0) > 0;
        
        Ok(HealthStatus {
            connected,
            response_time_ms,
            pool_size: self.pool.size() as u32,
            tables_exist,
        })
    }

    pub async fn initialize_default_assets(&self) -> Result<(), Box<dyn std::error::Error>> {
        use uuid::Uuid;
        
        let default_assets = vec![
            ("So11111111111111111111111111111111111111112", 9, "Solana", "SOL", Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png")),
            ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 6, "USD Coin", "USDC", Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png")),
            ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", 6, "Tether USD", "USDT", Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.png")),
            ("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", 9, "Marinade Staked SOL", "mSOL", None),
            ("7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs", 8, "Wrapped ETH (Wormhole)", "WETH", None),
        ];
        
        for (mint, decimals, name, symbol, logo) in default_assets {
            sqlx::query!(
                r#"
                INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
                ON CONFLICT (mint_address) DO UPDATE 
                SET decimals = EXCLUDED.decimals,
                    name = EXCLUDED.name,
                    symbol = EXCLUDED.symbol,
                    logo_url = COALESCE(EXCLUDED.logo_url, assets.logo_url),
                    updated_at = NOW()
                "#,
                Uuid::new_v4(),
                mint,
                decimals,
                name,
                symbol,
                logo.as_deref()
            )
            .execute(&self.pool)
            .await?;
        }
        
        Ok(())
    }

    pub async fn get_store_stats(&self) -> Result<StoreStats, Box<dyn std::error::Error>> {
        let users = self.get_user_stats().await?;
        let quotes = self.get_quote_stats().await?;
        
        let assets = sqlx::query_scalar!("SELECT COUNT(*) FROM assets")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);
        
        let balances = BalanceStats {
            total_balance_records: sqlx::query_scalar!("SELECT COUNT(*) FROM balances")
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0),
            non_zero_balances: sqlx::query_scalar!(
                "SELECT COUNT(*) FROM balances WHERE amount > 0"
            )
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0),
        };
        
        Ok(StoreStats {
            users,
            quotes,
            assets,
            balances,
        })
    }

    pub async fn maintenance_cleanup(&self) -> Result<MaintenanceResult, Box<dyn std::error::Error>> {
        let expired_quotes = self.cleanup_expired_quotes().await?;
        let old_used_quotes = self.cleanup_old_used_quotes(7).await?; // Clean quotes older than 7 days
        
        Ok(MaintenanceResult {
            expired_quotes_deleted: expired_quotes,
            old_quotes_deleted: old_used_quotes,
        })
    }

    pub async fn get_all_balances(&self, user_id: &Uuid) -> Result<Vec<BalanceWithAsset>, BalanceError> {
        let balances = sqlx::query!(
            r#"
            SELECT 
                COALESCE(b.amount, 0) as balance,
                a.mint_address as token_mint,
                a.symbol,
                a.decimals
            FROM assets a
            LEFT JOIN balances b ON a.id = b.asset_id AND b.user_id = $1
            ORDER BY a.symbol
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        let result = balances
            .into_iter()
            .map(|row| BalanceWithAsset {
                balance: row.balance.unwrap_or(0),
                token_mint: row.token_mint,
                symbol: row.symbol,
                decimals: row.decimals,
            })
            .collect();

        Ok(result)
    }
}

#[derive(Debug)]
pub struct HealthStatus {
    pub connected: bool,
    pub response_time_ms: u64,
    pub pool_size: u32,
    pub tables_exist: bool,
}

#[derive(Debug)]
pub struct StoreStats {
    pub users: UserStats,
    pub quotes: QuoteStats,
    pub assets: i64,
    pub balances: BalanceStats,
}

#[derive(Debug)]
pub struct BalanceStats {
    pub total_balance_records: i64,
    pub non_zero_balances: i64,
}

#[derive(Debug)]
pub struct MaintenanceResult {
    pub expired_quotes_deleted: u64,
    pub old_quotes_deleted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_store_creation() {

        if std::env::var("TEST_DATABASE_URL").is_ok() {
            let store = Store::from_env().await;
            assert!(store.is_ok());
            
            if let Ok(store) = store {
                let health = store.health_check().await;
                assert!(health.is_ok());
            }
        }
    }
    
    #[tokio::test]
    async fn test_detailed_health_check() {
        if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
            let store = Store::from_url(&database_url).await.unwrap();
            let health = store.detailed_health_check().await.unwrap();
            
            assert!(health.connected);
            assert!(health.response_time_ms < 1000);
            assert!(health.pool_size > 0);
        }
    }
}