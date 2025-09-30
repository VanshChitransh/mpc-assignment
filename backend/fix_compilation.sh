#!/bin/bash

echo "🔧 Fixing all compilation errors..."

cd /Users/vansh/Coding/SuperDevs/Assignments/purge-assignment/backend

# 1. Remove the problematic lib.rs file (not needed for binary crate)
echo "📝 Removing lib.rs (not needed for binary crate)..."
rm -f src/lib.rs

# 2. Fix the user.rs file - add missing imports
echo "📝 Fixing user.rs - adding missing imports..."
cat > src/routes/user.rs << 'EOF'
use actix_web::{get, post, web, HttpResponse, Result, HttpRequest, HttpMessage};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use tracing::{info, error};

// Request/Response types
#[derive(Deserialize)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub public_key: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserAuthResponse {
    pub success: bool,
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct UserErrorResponse {
    pub success: bool,
    pub error: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String, // user ID
    username: String, // email
    exp: i64, // expiry
    iat: i64, // issued at
}

// Helper trait to extract user ID from authenticated requests
trait AuthenticatedRequest {
    fn get_user_id(&self) -> Option<Uuid>;
}

impl AuthenticatedRequest for HttpRequest {
    fn get_user_id(&self) -> Option<Uuid> {
        // Extract user ID from JWT token stored in request extensions
        // This would be set by your auth middleware
        self.extensions().get::<Uuid>().copied()
    }
}

/// User signup endpoint
#[post("/signup")]
pub async fn sign_up(
    pool: web::Data<PgPool>,
    req_body: web::Json<SignUpRequest>,
) -> Result<HttpResponse> {
    let signup_req = req_body.into_inner();
    info!("User signup attempt: {}", signup_req.email);

    // Validate email format
    if !signup_req.email.contains('@') {
        return Ok(HttpResponse::BadRequest().json(UserErrorResponse {
            success: false,
            error: "Invalid email format".to_string(),
        }));
    }

    // Validate password length
    if signup_req.password.len() < 8 {
        return Ok(HttpResponse::BadRequest().json(UserErrorResponse {
            success: false,
            error: "Password must be at least 8 characters".to_string(),
        }));
    }

    // Check if user already exists
    match sqlx::query!(
        "SELECT email FROM users WHERE email = $1",
        signup_req.email.to_lowercase()
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(_)) => {
            return Ok(HttpResponse::BadRequest().json(UserErrorResponse {
                success: false,
                error: "User already exists".to_string(),
            }));
        }
        Ok(None) => {} // User doesn't exist, continue
        Err(e) => {
            error!("Database error during user check: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Database error".to_string(),
            }));
        }
    }

    // Hash password
    let password_hash = match hash(signup_req.password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Password hashing error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Password processing error".to_string(),
            }));
        }
    };

    // Create user
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    match sqlx::query!(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        user_id,
        signup_req.email.to_lowercase(),
        password_hash,
        now,
        now
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            info!("Successfully created user {}: {}", user_id, signup_req.email);
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Failed to create user".to_string(),
            }));
        }
    }

    // Generate JWT token
    let token = generate_jwt_token(&user_id.to_string(), &signup_req.email)?;

    let user_response = UserResponse {
        id: user_id.to_string(),
        email: signup_req.email,
        public_key: None, // Will be set after MPC key generation
        created_at: now,
    };

    Ok(HttpResponse::Created().json(UserAuthResponse {
        success: true,
        token,
        user: user_response,
    }))
}

/// User signin endpoint
#[post("/signin")]
pub async fn sign_in(
    pool: web::Data<PgPool>,
    req_body: web::Json<SignInRequest>,
) -> Result<HttpResponse> {
    let signin_req = req_body.into_inner();
    info!("User signin attempt: {}", signin_req.email);

    // Get user from database
    let user = match sqlx::query!(
        "SELECT id, email, password_hash, public_key, created_at FROM users WHERE email = $1",
        signin_req.email.to_lowercase()
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(UserErrorResponse {
                success: false,
                error: "Invalid credentials".to_string(),
            }));
        }
        Err(e) => {
            error!("Database error during signin: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Database error".to_string(),
            }));
        }
    };

    // Verify password
    match verify(&signin_req.password, &user.password_hash) {
        Ok(true) => {
            info!("Successful signin for user {}: {}", user.id, user.email);
        }
        Ok(false) => {
            return Ok(HttpResponse::Unauthorized().json(UserErrorResponse {
                success: false,
                error: "Invalid credentials".to_string(),
            }));
        }
        Err(e) => {
            error!("Password verification error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Authentication error".to_string(),
            }));
        }
    }

    // Generate JWT token
    let token = generate_jwt_token(&user.id.to_string(), &user.email)?;

    let user_response = UserResponse {
        id: user.id.to_string(),
        email: user.email,
        public_key: user.public_key,
        created_at: user.created_at,
    };

    Ok(HttpResponse::Ok().json(UserAuthResponse {
        success: true,
        token,
        user: user_response,
    }))
}

/// Get user profile (protected route)
#[get("/profile")]
pub async fn get_profile(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::Unauthorized().json(UserErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };

    info!("Getting profile for user {}", user_id);

    let user = match sqlx::query!(
        "SELECT id, email, public_key, created_at FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(UserErrorResponse {
                success: false,
                error: "User not found".to_string(),
            }));
        }
        Err(e) => {
            error!("Database error getting user profile: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Database error".to_string(),
            }));
        }
    };

    let user_response = UserResponse {
        id: user.id.to_string(),
        email: user.email,
        public_key: user.public_key,
        created_at: user.created_at,
    };

    Ok(HttpResponse::Ok().json(user_response))
}

fn generate_jwt_token(user_id: &str, email: &str) -> Result<String> {
    let now = Utc::now();
    let exp = now + Duration::hours(24); // Token expires in 24 hours

    let claims = Claims {
        sub: user_id.to_string(),
        username: email.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    // TODO: Use a proper secret from environment variables
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret".to_string());
    
    match encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())) {
        Ok(token) => Ok(token),
        Err(e) => {
            error!("JWT encoding error: {}", e);
            Err(actix_web::error::ErrorInternalServerError("Token generation error"))
        }
    }
}
EOF

# 3. Fix the solana.rs file - fix SQLx issues and add missing imports
echo "📝 Fixing solana.rs - fixing SQLx queries and adding imports..."
cat > src/routes/solana.rs << 'EOF'
use actix_web::{get, post, web, HttpResponse, Result, HttpRequest, HttpMessage};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use tracing::{info, error};

// Request/Response types
#[derive(Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: String,
    pub slippage_bps: Option<u16>,
}

#[derive(Deserialize)]
pub struct SwapRequest {
    pub quote_id: String,
}

#[derive(Deserialize)]
pub struct SendTokenRequest {
    pub to_address: String,
    pub mint_address: String,
    pub amount: String,
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub success: bool,
    pub balances: Vec<TokenBalance>,
}

#[derive(Serialize)]
pub struct TokenBalance {
    pub mint: String,
    pub amount: String,
    pub symbol: String,
    pub decimals: i32, // Changed from u8 to i32 for SQLx compatibility
}

#[derive(Serialize)]
pub struct QuoteResponse {
    pub success: bool,
    pub quote_id: String,
    pub out_amount: String,
    pub price_impact_pct: f64,
}

#[derive(Serialize)]
pub struct SwapResponse {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SendResponse {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SolanaErrorResponse {
    pub success: bool,
    pub error: String,
}

// Helper trait to extract user ID from authenticated requests
trait AuthenticatedRequest {
    fn get_user_id(&self) -> Option<Uuid>;
}

impl AuthenticatedRequest for HttpRequest {
    fn get_user_id(&self) -> Option<Uuid> {
        // Extract user ID from JWT token stored in request extensions
        // This would be set by your auth middleware
        self.extensions().get::<Uuid>().copied()
    }
}

/// Get user's token balances
#[get("/balance")]
pub async fn get_balance(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::Unauthorized().json(SolanaErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };

    info!("Getting balances for user {}", user_id);

    // Use a simpler query approach that works with SQLx
    let balance_rows = match sqlx::query!(
        r#"
        SELECT 
            a.mint_address,
            b.amount,
            a.symbol,
            a.decimals
        FROM balances b
        JOIN assets a ON b.asset_id = a.id
        WHERE b.user_id = $1 AND b.amount > 0
        ORDER BY a.symbol
        "#,
        user_id
    )
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            error!("Failed to fetch balances for user {}: {}", user_id, e);
            return Ok(HttpResponse::InternalServerError().json(SolanaErrorResponse {
                success: false,
                error: "Failed to fetch balances".to_string(),
            }));
        }
    };

    // Convert the rows to TokenBalance structs
    let balances: Vec<TokenBalance> = balance_rows
        .into_iter()
        .map(|row| TokenBalance {
            mint: row.mint_address,
            amount: row.amount.to_string(),
            symbol: row.symbol,
            decimals: row.decimals,
        })
        .collect();

    Ok(HttpResponse::Ok().json(BalanceResponse {
        success: true,
        balances,
    }))
}

/// Get swap quote from Jupiter
#[post("/quote")]
pub async fn get_quote(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    req_body: web::Json<QuoteRequest>,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::Unauthorized().json(SolanaErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };

    let quote_req = req_body.into_inner();
    info!("Getting quote for user {}: {} -> {}", 
          user_id, quote_req.input_mint, quote_req.output_mint);

    // For now, return a mock quote
    // TODO: Replace with actual Jupiter API call
    let mock_quote_id = Uuid::new_v4();
    let mock_out_amount = "1000000"; // 1 USDC (6 decimals)
    let mock_price_impact = 0.1;

    // Store quote in database
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(5);
    
    match sqlx::query!(
        r#"
        INSERT INTO quotes (id, user_id, input_mint, output_mint, in_amount, out_amount, quote_data, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        mock_quote_id,
        user_id,
        quote_req.input_mint,
        quote_req.output_mint,
        quote_req.amount.parse::<i64>().unwrap_or(0),
        mock_out_amount.parse::<i64>().unwrap_or(0),
        json!({"mock": true}),
        expires_at
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            info!("Successfully created quote {} for user {}", mock_quote_id, user_id);
        }
        Err(e) => {
            error!("Failed to store quote: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SolanaErrorResponse {
                success: false,
                error: "Failed to create quote".to_string(),
            }));
        }
    }

    Ok(HttpResponse::Ok().json(QuoteResponse {
        success: true,
        quote_id: mock_quote_id.to_string(),
        out_amount: mock_out_amount.to_string(),
        price_impact_pct: mock_price_impact,
    }))
}

/// Execute swap transaction
#[post("/swap")]
pub async fn execute_swap(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    req_body: web::Json<SwapRequest>,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::Unauthorized().json(SolanaErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };

    let swap_req = req_body.into_inner();
    info!("Executing swap for user {}: quote_id {}", user_id, swap_req.quote_id);

    // Parse quote ID
    let quote_id = match Uuid::parse_str(&swap_req.quote_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(SolanaErrorResponse {
                success: false,
                error: "Invalid quote ID format".to_string(),
            }));
        }
    };

    // Validate quote exists and is not expired
    let _quote_validation = match sqlx::query!(
        "SELECT id FROM quotes WHERE id = $1 AND user_id = $2 AND expires_at > NOW() AND used = false",
        quote_id,
        user_id
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(_)) => {
            info!("Quote {} validated for user {}", quote_id, user_id);
        }
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(SolanaErrorResponse {
                success: false,
                error: "Quote not found or expired".to_string(),
            }));
        }
        Err(e) => {
            error!("Failed to validate quote: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SolanaErrorResponse {
                success: false,
                error: "Failed to validate quote".to_string(),
            }));
        }
    };

    // Mark quote as used
    match sqlx::query!(
        "UPDATE quotes SET used = true, used_at = NOW() WHERE id = $1",
        quote_id
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            info!("Marked quote {} as used", quote_id);
        }
        Err(e) => {
            error!("Failed to mark quote as used: {}", e);
        }
    }

    // TODO: Replace with actual MPC signing and transaction broadcasting
    let mock_signature = "5uJL9xxxx...mock_transaction_signature";

    Ok(HttpResponse::Ok().json(SwapResponse {
        success: true,
        transaction_signature: Some(mock_signature.to_string()),
        error: None,
    }))
}

/// Send tokens to another address
#[post("/send")]
pub async fn send_tokens(
    _pool: web::Data<PgPool>, // Prefixed with _ to avoid unused warning
    req: HttpRequest,
    req_body: web::Json<SendTokenRequest>,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::Unauthorized().json(SolanaErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };

    let send_req = req_body.into_inner();
    info!("Sending tokens for user {}: {} {} to {}", 
          user_id, send_req.amount, send_req.mint_address, send_req.to_address);

    // TODO: Add balance validation, MPC signing, and transaction broadcasting
    let mock_signature = "5uJL9xxxx...mock_send_signature";

    Ok(HttpResponse::Ok().json(SendResponse {
        success: true,
        transaction_signature: Some(mock_signature.to_string()),
        error: None,
    }))
}
EOF

# 4. Fix the routes/mod.rs to avoid naming conflicts
echo "📝 Fixing routes/mod.rs to avoid naming conflicts..."
cat > src/routes/mod.rs << 'EOF'
pub mod user;
pub mod solana;
pub mod health;

// Export specific types to avoid conflicts
pub use user::{sign_up, sign_in, get_profile};
pub use solana::{get_balance, get_quote, execute_swap, send_tokens};
pub use health::health_check;
EOF

echo "✅ All compilation errors fixed!"

# 5. Test the build
echo "🧪 Testing build..."
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"

if cargo build; then
    echo "🎉 Backend builds successfully!"
    echo ""
    echo "✅ Ready to run!"
    echo "Run: cargo run"
else
    echo "❌ Build still has issues. Check the errors above."
fi