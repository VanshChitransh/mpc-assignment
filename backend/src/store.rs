// backend/src/store.rs
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub public_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub id: Uuid,
    pub user_id: Uuid,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: i64,
    pub out_amount: i64,
    pub quote_data: serde_json::Value,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Clone)]
pub struct Store {
    pub pool: PgPool,
}

impl Store {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<User, sqlx::Error> {
        let user_id = Uuid::parse_str(id).map_err(|_| {
            sqlx::Error::Protocol("Invalid UUID format".into())
        })?;
        
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password_hash, public_key, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }
    
    pub async fn update_user_public_key(&self, user_id: &str, public_key: &str) -> Result<(), sqlx::Error> {
        let user_id = Uuid::parse_str(user_id).map_err(|_| {
            sqlx::Error::Protocol("Invalid UUID format".into())
        })?;
        
        sqlx::query!(
            r#"
            UPDATE users
            SET public_key = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            public_key,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    
    pub async fn create_quote(
        &self, 
        user_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
        out_amount: u64,
        quote_data: serde_json::Value,
        expires_at: DateTime<Utc>
    ) -> Result<Quote, sqlx::Error> {
        let user_id = Uuid::parse_str(user_id).map_err(|_| {
            sqlx::Error::Protocol("Invalid UUID format".into())
        })?;
        
        let quote_id = Uuid::new_v4();
        
        let quote = sqlx::query_as!(
            Quote,
            r#"
            INSERT INTO quotes (id, user_id, input_mint, output_mint, in_amount, out_amount, quote_data, expires_at, used, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, NOW())
            RETURNING id, user_id, input_mint, output_mint, in_amount, out_amount, quote_data, expires_at, used, created_at
            "#,
            quote_id,
            user_id,
            input_mint,
            output_mint,
            in_amount as i64, // Convert to i64 for database
            out_amount as i64, // Convert to i64 for database
            quote_data,
            expires_at
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(quote)
    }
    
    pub async fn get_valid_quote(&self, quote_id: &Uuid, user_id: &str) -> Result<Quote, sqlx::Error> {
        let user_id = Uuid::parse_str(user_id).map_err(|_| {
            sqlx::Error::Protocol("Invalid UUID format".into())
        })?;
        
        let quote = sqlx::query_as!(
            Quote,
            r#"
            SELECT id, user_id, input_mint, output_mint, in_amount, out_amount, quote_data, expires_at, used, created_at
            FROM quotes
            WHERE id = $1 AND user_id = $2 AND expires_at > NOW() AND used = false
            "#,
            quote_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(quote)
    }
    
    pub async fn mark_quote_used(&self, quote_id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE quotes
            SET used = true
            WHERE id = $1
            "#,
            quote_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn get_asset_by_mint(&self, mint_address: &str) -> Result<Asset, sqlx::Error> {
        let asset = sqlx::query_as!(
            Asset,
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets
            WHERE mint_address = $1
            "#,
            mint_address
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(asset)
    }
}