use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)] 
    pub password_hash: String,
    pub public_key: Option<String>, 
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
}


#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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


#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Balance {
    pub id: Uuid,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub asset_id: Uuid,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceWithAsset {
    pub balance: i64,
    pub token_mint: String,
    pub symbol: String,
    pub decimals: i32,
}


#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Quote {
    pub id: Uuid,
    pub user_id: Uuid,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: i64,
    pub out_amount: i64,
    pub price_impact_pct: Option<rust_decimal::Decimal>,
    pub quote_data: serde_json::Value,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used: bool,
    pub updated_at: DateTime<Utc>,
}


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Keyshare {
    pub user_id: Uuid,
    pub public_key: String,
    pub private_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
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




#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("User exists: {0}")]
    UserExists(String),
    #[error("Password hash failed: {0}")]
    PasswordHashFailed(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
}

#[derive(Debug, thiserror::Error)]
pub enum BalanceError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Balance not found")]
    BalanceNotFound,
    #[error("Insufficient balance")]
    InsufficientBalance { required: u64, available: u64 },
    #[error("Asset not found: {0}")]
    AssetNotFound(Uuid),
    #[error("Asset not found by mint: {0}")]
    AssetNotFoundByMint(String),
    #[error("Asset not found by symbol: {0}")]
    AssetNotFoundBySymbol(String),
}#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: i64,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: Option<i32>, 
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    #[serde(rename = "outAmount")]
    pub out_amount: i64,
    pub price_impact_pct: Option<rust_decimal::Decimal>,
    pub id: String,
    pub slippage_bps: i32,
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
    pub mint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendResponse {
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct SolBalanceResponse {
    pub balance: i64,
    pub balance_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct TokenBalancesResponse {
    pub balances: Vec<BalanceWithAsset>,
}

#[derive(Debug, Serialize)]
pub struct AllBalancesResponse {
    pub sol_balance: i64,
    pub sol_balance_formatted: f64,
    pub token_balances: Vec<BalanceWithAsset>,
    pub total_assets: usize,
}


#[derive(Debug, Deserialize)]
pub struct SignUpRequest {
    pub username: String,
    pub email: String, 
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SignInRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Serialize)]
pub struct SignUpResponse {
    pub message: String,
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub public_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub has_mpc_keys: bool,
}

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub email: String,
    pub public_key: Option<String>,
    pub wallet_address: Option<String>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct MpcKeyGenRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MpcKeyGenResponse {
    pub user_id: Uuid,
    pub public_key: String,
    pub success: bool,
    pub node_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MpcSignRequest {
    pub user_id: Uuid,
    pub transaction_data: Vec<u8>,
    pub message: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MpcSignResponse {
    pub user_id: Uuid,
    pub signature: Vec<u8>,
    pub success: bool,
    pub node_id: String,
}



impl User {
    pub fn new(email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash,
            public_key: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn to_profile(&self) -> UserProfile {
        UserProfile {
            id: self.id,
            email: self.email.clone(),
            public_key: self.public_key.clone(),
            created_at: self.created_at,
            has_mpc_keys: self.public_key.is_some(),
        }
    }
}

impl Balance {
    pub fn new(user_id: Uuid, asset_id: Uuid, amount: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
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
            price_impact_pct: None,            updated_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

impl Asset {
    pub fn new(mint_address: String, decimals: i32, name: String, symbol: String, logo_url: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            mint_address,
            decimals,
            name,
            symbol,
            logo_url,
            created_at: now,
            updated_at: now,
        }
    }
}

impl BalanceWithAsset {

    pub fn to_human_readable(&self) -> f64 {
        self.balance as f64 / 10_f64.powi(self.decimals)
    }


    pub fn formatted_balance(&self) -> String {
        let human = self.to_human_readable();
        if self.decimals <= 2 {
            format!("{:.2}", human)
        } else if self.decimals <= 6 {
            format!("{:.6}", human)
        } else {
            format!("{:.9}", human)
        }
    }
}


#[macro_export]
macro_rules! db_error {
    ($expr:expr) => {
        $expr.map_err(|e| BalanceError::DatabaseError(e.to_string()))
    };
}

#[macro_export]
macro_rules! user_db_error {
    ($expr:expr) => {
        $expr.map_err(|e| UserError::DatabaseError(e.to_string()))
    };
}

#[macro_export]
macro_rules! quote_db_error {
    ($expr:expr) => {
        $expr.map_err(|e| QuoteError::DatabaseError(e.to_string()))
    };
}

pub const LAMPORTS_PER_SOL: i64 = 1_000_000_000;
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

pub const DEFAULT_QUOTE_EXPIRATION_SECONDS: i64 = 300;