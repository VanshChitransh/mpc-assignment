pub mod user;
pub mod models;
pub mod balance;
pub mod quote;

use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub use models::*;

pub struct Store {
    pub pool: PgPool,
}

impl Store {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new database connection pool
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

    /// Run database migrations
    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("../migrations").run(&self.pool).await
    }

    /// Create a new store instance with database pool
    pub async fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        
        let pool = Self::new_pool(&database_url).await?;
        let store = Self::new(pool);
        
        // Run migrations
        store.migrate().await?;
        
        Ok(store)
    }

    /// Health check for the database connection
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_store_creation() {
        // This test requires a DATABASE_URL environment variable
        if std::env::var("DATABASE_URL").is_ok() {
            let store = Store::from_env().await;
            assert!(store.is_ok());
            
            if let Ok(store) = store {
                let health = store.health_check().await;
                assert!(health.is_ok());
            }
        }
    }
}