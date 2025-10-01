use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use thiserror::Error;
use tracing::{info, error};

use crate::{Asset, AssetError};

#[derive(Debug, Error)]
pub enum BalanceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Asset not found: {0}")]
    AssetNotFound(String),
    
    #[error("User not found: {0}")]
    UserNotFound(Uuid),
}

#[derive(Debug, Clone)]
pub struct Balance {
    pub id: Uuid,
    pub user_id: Uuid,
    pub asset_id: Uuid,
    pub amount: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct BalanceWithAsset {
    pub balance: Balance,
    pub asset: Asset,
}

pub struct BalanceStore<'a> {
    pool: &'a PgPool,
}

impl<'a> BalanceStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
    
    /// Get balance for a specific user and asset
    pub async fn get_balance(&self, user_id: &Uuid, asset_id: &Uuid) -> Result<Option<Balance>, BalanceError> {
        let balance = sqlx::query_as!(
            Balance,
            r#"
            SELECT id, user_id, asset_id, amount, created_at, updated_at
            FROM balances
            WHERE user_id = $1 AND asset_id = $2
            "#,
            user_id,
            asset_id
        )
        .fetch_optional(self.pool)
        .await?;
        
        Ok(balance)
    }
    
    /// Get balance for a specific user and mint address
    pub async fn get_balance_by_mint(&self, user_id: &Uuid, mint_address: &str) -> Result<Option<BalanceWithAsset>, BalanceError> {
        // Fixed SQL query to avoid duplicate field names
        let result = sqlx::query!(
            r#"
            SELECT 
                b.id, b.user_id, b.asset_id, b.amount, b.created_at, b.updated_at,
                a.id as asset_id_val, a.mint_address, a.decimals, a.name, a.symbol, a.logo_url, 
                a.created_at as asset_created_at, a.updated_at as asset_updated_at
            FROM balances b
            JOIN assets a ON b.asset_id = a.id
            WHERE b.user_id = $1 AND a.mint_address = $2
            "#,
            user_id,
            mint_address
        )
        .fetch_optional(self.pool)
        .await?;
        
        match result {
            Some(row) => {
                let balance = Balance {
                    id: row.id,
                    user_id: row.user_id,
                    asset_id: row.asset_id,
                    amount: row.amount,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                
                let asset = Asset {
                    id: row.asset_id_val,
                    mint_address: row.mint_address,
                    decimals: row.decimals,
                    name: row.name,
                    symbol: row.symbol,
                    logo_url: row.logo_url,
                    created_at: row.asset_created_at,
                    updated_at: row.asset_updated_at,
                };
                
                Ok(Some(BalanceWithAsset { balance, asset }))
            },
            None => Ok(None)
        }
    }
    
    /// Get all balances for a user
    pub async fn get_all_balances(&self, user_id: &Uuid) -> Result<Vec<BalanceWithAsset>, BalanceError> {
        // Fixed SQL query to avoid duplicate field names
        let rows = sqlx::query!(
            r#"
            SELECT 
                b.id, b.user_id, b.asset_id, b.amount, b.created_at, b.updated_at,
                a.id as asset_id_val, a.mint_address, a.decimals, a.name, a.symbol, a.logo_url, 
                a.created_at as asset_created_at, a.updated_at as asset_updated_at
            FROM balances b
            JOIN assets a ON b.asset_id = a.id
            WHERE b.user_id = $1
            "#,
            user_id
        )
        .fetch_all(self.pool)
        .await?;
        
        let balances = rows.into_iter().map(|row| {
            let balance = Balance {
                id: row.id,
                user_id: row.user_id,
                asset_id: row.asset_id,
                amount: row.amount,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };
            
            let asset = Asset {
                id: row.asset_id_val,
                mint_address: row.mint_address,
                decimals: row.decimals,
                name: row.name,
                symbol: row.symbol,
                logo_url: row.logo_url,
                created_at: row.asset_created_at,
                updated_at: row.asset_updated_at,
            };
            
            BalanceWithAsset { balance, asset }
        }).collect();
        
        Ok(balances)
    }
    
    /// Update or create a balance
    pub async fn update_balance(&self, user_id: &Uuid, asset_id: &Uuid, amount: i64) -> Result<Balance, BalanceError> {
        let balance = sqlx::query_as!(
            Balance,
            r#"
            INSERT INTO balances (user_id, asset_id, amount)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, asset_id)
            DO UPDATE SET amount = $3, updated_at = NOW()
            RETURNING id, user_id, asset_id, amount, created_at, updated_at
            "#,
            user_id,
            asset_id,
            amount
        )
        .fetch_one(self.pool)
        .await?;
        
        Ok(balance)
    }
    
    /// Update or create a balance by mint address
    pub async fn update_balance_by_mint(&self, user_id: &Uuid, mint_address: &str, amount: i64) -> Result<BalanceWithAsset, BalanceError> {
        // Start transaction
        let mut tx = self.pool.begin().await?;
        
        // Get asset or create if not exists
        let asset = self.get_or_create_asset_tx(&mut tx, mint_address).await?;
        
        // Update balance
        let balance = sqlx::query_as!(
            Balance,
            r#"
            INSERT INTO balances (user_id, asset_id, amount)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, asset_id)
            DO UPDATE SET amount = $3, updated_at = NOW()
            RETURNING id, user_id, asset_id, amount, created_at, updated_at
            "#,
            user_id,
            asset.id,
            amount
        )
        .fetch_one(&mut *tx)
        .await?;
        
        // Commit transaction
        tx.commit().await?;
        
        Ok(BalanceWithAsset { balance, asset })
    }
    
    /// Increment or decrement a balance
    pub async fn increment_balance(&self, user_id: &Uuid, asset_id: &Uuid, amount: i64) -> Result<Balance, BalanceError> {
        let balance = sqlx::query_as!(
            Balance,
            r#"
            INSERT INTO balances (user_id, asset_id, amount)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, asset_id)
            DO UPDATE SET amount = balances.amount + $3, updated_at = NOW()
            RETURNING id, user_id, asset_id, amount, created_at, updated_at
            "#,
            user_id,
            asset_id,
            amount
        )
        .fetch_one(self.pool)
        .await?;
        
        Ok(balance)
    }
    
    /// Get or create asset within a transaction
    async fn get_or_create_asset_tx<'b>(&self, tx: &mut Transaction<'b, Postgres>, mint_address: &str) 
        -> Result<Asset, BalanceError> 
    {
        // First, try to find existing asset
        let asset = sqlx::query_as!(
            Asset,
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets
            WHERE mint_address = $1
            "#,
            mint_address
        )
        .fetch_optional(&mut **tx)
        .await?;
        
        if let Some(asset) = asset {
            return Ok(asset);
        }
        
        // If not found, create a new one with minimal info
        let asset = sqlx::query_as!(
            Asset,
            r#"
            INSERT INTO assets (mint_address, decimals, name, symbol)
            VALUES ($1, $2, $3, $4)
            RETURNING id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            "#,
            mint_address,
            0, // Default decimals (will be updated later)
            format!("Unknown Token ({})", truncate_address(mint_address)),
            truncate_address(mint_address)
        )
        .fetch_one(&mut **tx)
        .await?;
        
        Ok(asset)
    }
}

/// Helper function to truncate an address for display
fn truncate_address(address: &str) -> String {
    if address.len() <= 8 {
        return address.to_string();
    }
    
    let start = &address[0..4];
    let end = &address[address.len() - 4..];
    format!("{}...{}", start, end)
}