use sqlx::PgPool;
use uuid::Uuid;
use thiserror::Error;
use tracing::{info, error};
use chrono::{DateTime, Utc};

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Asset not found: {0}")]
    AssetNotFound(String),
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub id: Uuid,
    pub mint_address: String,
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AssetStore<'a> {
    pool: &'a PgPool,
}

impl<'a> AssetStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
    
    /// Get asset by ID
    pub async fn get_asset(&self, id: &Uuid) -> Result<Asset, AssetError> {
        let asset = sqlx::query_as!(
            Asset,
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool)
        .await?;
        
        match asset {
            Some(asset) => Ok(asset),
            None => Err(AssetError::AssetNotFound(id.to_string())),
        }
    }
    
    /// Get asset by mint address
    pub async fn get_asset_by_mint(&self, mint_address: &str) -> Result<Asset, AssetError> {
        let asset = sqlx::query_as!(
            Asset,
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets
            WHERE mint_address = $1
            "#,
            mint_address
        )
        .fetch_optional(self.pool)
        .await?;
        
        match asset {
            Some(asset) => Ok(asset),
            None => Err(AssetError::AssetNotFound(mint_address.to_string())),
        }
    }
    
    /// Get or create an asset
    pub async fn get_or_create_asset(&self, mint_address: &str, decimals: i32, name: Option<String>, symbol: Option<String>) 
        -> Result<Asset, AssetError> 
    {
        // Try to get existing asset
        let existing = sqlx::query_as!(
            Asset,
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets
            WHERE mint_address = $1
            "#,
            mint_address
        )
        .fetch_optional(self.pool)
        .await?;
        
        if let Some(asset) = existing {
            return Ok(asset);
        }
        
        // Create new asset
        let default_name = format!("Unknown Token ({})", truncate_address(mint_address));
        let default_symbol = truncate_address(mint_address);
        
        let new_asset = sqlx::query_as!(
            Asset,
            r#"
            INSERT INTO assets (mint_address, decimals, name, symbol)
            VALUES ($1, $2, $3, $4)
            RETURNING id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            "#,
            mint_address,
            decimals,
            name.unwrap_or(default_name),
            symbol.unwrap_or(default_symbol)
        )
        .fetch_one(self.pool)
        .await?;
        
        Ok(new_asset)
    }
    
    /// Update asset information
    pub async fn update_asset(&self, id: &Uuid, decimals: Option<i32>, name: Option<String>, symbol: Option<String>, logo_url: Option<String>) 
        -> Result<Asset, AssetError> 
    {
        // Get current asset
        let current = self.get_asset(id).await?;
        
        // Update with new values or keep current ones
        let updated = sqlx::query_as!(
            Asset,
            r#"
            UPDATE assets
            SET 
                decimals = $2,
                name = $3,
                symbol = $4,
                logo_url = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            "#,
            id,
            decimals.unwrap_or(current.decimals),
            name.unwrap_or(current.name),
            symbol.unwrap_or(current.symbol),
            logo_url.as_deref(),
        )
        .fetch_one(self.pool)
        .await?;
        
        Ok(updated)
    }
    
    /// Get all assets
    pub async fn get_all_assets(&self) -> Result<Vec<Asset>, AssetError> {
        let assets = sqlx::query_as!(
            Asset,
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets
            ORDER BY name ASC
            "#,
        )
        .fetch_all(self.pool)
        .await?;
        
        Ok(assets)
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