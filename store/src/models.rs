use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

// User model matching the database schema
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)] // Don't expose password hash in JSON
    pub password_hash: String,
    pub public_key: Option<String>, // Solana public key from MPC
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// User creation request
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
}

// Asset model for supported tokens
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Asset {
    pub id: String,
    pub mint_address: String,
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Balance model for user token balances
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Balance {
    pub id: String,
    pub amount: i64, // Stored in smallest units (lamports for SOL)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: String,
    pub asset_id: String,
}

// Extended balance with asset information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceWithAsset {
    pub balance: i64,
    pub token_mint: String,
    pub symbol: String,
    pub decimals: i32,
}

// Quote model for Jupiter swap quotes
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Quote {
    pub id: Uuid,
    pub user_id: Uuid,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: i64,
    pub out_amount: i64,
    pub quote_data: serde_json::Value, // Full Jupiter quote response as JSON
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Keyshare model for MPC nodes
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Keyshare {
    pub user_id: String,
    pub public_key: String,
    pub private_key: String, // Encrypted key share
    pub created_at: DateTime<Utc>,
}

// Request/Response types for API endpoints

#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: i64,
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    #[serde(rename = "outAmount")]
    pub out_amount: i64,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct SwapRequest {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct SwapResponse {
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub to: String,
    pub amount: i64,
    pub mint: Option<String>, // None for SOL, Some(mint_address) for tokens
}

#[derive(Debug, Serialize)]
pub struct SendResponse {
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct SolBalanceResponse {
    pub balance: i64, // in lamports
}

#[derive(Debug, Serialize)]
pub struct TokenBalancesResponse {
    pub balances: Vec<BalanceWithAsset>,
}

// User authentication and profile types
#[derive(Debug, Deserialize)]
pub struct SignUpRequest {
    pub username: String, // API spec uses "username" but it's actually email
    pub email: String, 
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SignInRequest {
    pub username: String, // API spec uses "username" but it's actually email
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct SignUpResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub email: String,
}

// MPC communication types
#[derive(Debug, Serialize, Deserialize)]
pub struct MpcKeyGenRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MpcKeyGenResponse {
    pub user_id: String,
    pub public_key: String,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MpcSignRequest {
    pub user_id: String,
    pub transaction_data: Vec<u8>,
    pub message: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MpcSignResponse {
    pub user_id: String,
    pub signature: Vec<u8>,
    pub success: bool,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("User already exists")]
    UserExists,
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum BalanceError {
    #[error("Balance not found")]
    BalanceNotFound,
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Asset not found")]
    AssetNotFound,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum QuoteError {
    #[error("Quote not found")]
    QuoteNotFound,
    #[error("Quote expired")]
    QuoteExpired,
    #[error("Quote already used")]
    QuoteAlreadyUsed,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

// Helper functions for creating new instances
impl User {
    pub fn new(email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            password_hash,
            public_key: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl Balance {
    pub fn new(user_id: String, asset_id: String, amount: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            amount,
            created_at: now,
            updated_at: now,
            user_id,
            asset_id,
        }
    }
}

impl Quote {
    pub fn new(
        user_id: Uuid,
        input_mint: String,
        output_mint: String,
        in_amount: i64,
        out_amount: i64,
        quote_data: serde_json::Value,
        expires_in_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        Quote {
            id: Uuid::new_v4(),
            user_id,
            input_mint,
            output_mint,
            in_amount,
            out_amount,
            quote_data,
            expires_at: now + chrono::Duration::seconds(expires_in_seconds),
            created_at: now,
            used: false,
            updated_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}