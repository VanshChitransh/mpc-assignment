#!/bin/bash

echo "🔧 Fixing backend compilation issues..."

cd /Users/vansh/Coding/SuperDevs/Assignments/purge-assignment/backend

# 1. Create the fixed main.rs file
echo "📝 Creating main.rs..."
cat > src/main.rs << 'EOF'
use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use env_logger::Env;
use sqlx::PgPool;
use std::env;

mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Create database connection pool
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let bind_address = "127.0.0.1:8080";
    println!("🚀 Starting server at http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/user")
                            .service(routes::user::sign_up)
                            .service(routes::user::sign_in)
                            .service(routes::user::get_profile)
                    )
                    .service(
                        web::scope("/solana")
                            .service(routes::solana::get_balance)
                            .service(routes::solana::get_quote)
                            .service(routes::solana::execute_swap)
                            .service(routes::solana::send_tokens)
                    )
            )
            .service(routes::health::health_check)
    })
    .bind(bind_address)?
    .run()
    .await
}
EOF

# 2. Create the basic solana.rs routes
echo "📝 Creating solana.rs routes..."
cat > src/routes/solana.rs << 'EOF'
use actix_web::{get, post, web, HttpResponse, Result, HttpRequest};
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
    pub decimals: u8,
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
pub struct ErrorResponse {
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
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };

    info!("Getting balances for user {}", user_id);

    // Query user balances from database
    let balances = match sqlx::query_as!(
        TokenBalance,
        r#"
        SELECT 
            a.mint_address as mint,
            b.amount::text as amount,
            a.symbol,
            a.decimals as "decimals!: u8"
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
        Ok(balances) => balances,
        Err(e) => {
            error!("Failed to fetch balances for user {}: {}", user_id, e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to fetch balances".to_string(),
            }));
        }
    };

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
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
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
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
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
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
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
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid quote ID format".to_string(),
            }));
        }
    };

    // Validate quote exists and is not expired
    let quote = match sqlx::query!(
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
            return Ok(HttpResponse::NotFound().json(ErrorResponse {
                success: false,
                error: "Quote not found or expired".to_string(),
            }));
        }
        Err(e) => {
            error!("Failed to validate quote: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
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
    pool: web::Data<PgPool>,
    req: HttpRequest,
    req_body: web::Json<SendTokenRequest>,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
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

# 3. Create fixed user.rs routes
echo "📝 Creating user.rs routes..."
cat > src/routes/user.rs << 'EOF'
use actix_web::{get, post, web, HttpResponse, Result, HttpRequest};
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
pub struct AuthResponse {
    pub success: bool,
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct ErrorResponse {
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
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            success: false,
            error: "Invalid email format".to_string(),
        }));
    }

    // Validate password length
    if signup_req.password.len() < 8 {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
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
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "User already exists".to_string(),
            }));
        }
        Ok(None) => {} // User doesn't exist, continue
        Err(e) => {
            error!("Database error during user check: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
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
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
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
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to create user".to_string(),
            }));
        }
    }

    // TODO: Trigger MPC key generation here
    // For now, we'll create the user without a public key

    // Generate JWT token
    let token = generate_jwt_token(&user_id.to_string(), &signup_req.email)?;

    let user_response = UserResponse {
        id: user_id.to_string(),
        email: signup_req.email,
        public_key: None, // Will be set after MPC key generation
        created_at: now,
    };

    Ok(HttpResponse::Created().json(AuthResponse {
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
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Invalid credentials".to_string(),
            }));
        }
        Err(e) => {
            error!("Database error during signin: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
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
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Invalid credentials".to_string(),
            }));
        }
        Err(e) => {
            error!("Password verification error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
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

    Ok(HttpResponse::Ok().json(AuthResponse {
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
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
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
            return Ok(HttpResponse::NotFound().json(ErrorResponse {
                success: false,
                error: "User not found".to_string(),
            }));
        }
        Err(e) => {
            error!("Database error getting user profile: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
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

# 4. Update the routes/mod.rs file
echo "📝 Updating routes/mod.rs..."
cat > src/routes/mod.rs << 'EOF'
pub mod user;
pub mod solana;
pub mod health;

pub use user::*;
pub use solana::*;
pub use health::*;
EOF

# 5. Add missing dependencies to Cargo.toml
echo "📝 Adding missing dependencies to Cargo.toml..."

# Check if jsonwebtoken is already in Cargo.toml
if ! grep -q "jsonwebtoken" Cargo.toml; then
    echo 'jsonwebtoken = "9"' >> Cargo.toml
fi

if ! grep -q "chrono" Cargo.toml; then
    echo 'chrono = { version = "0.4", features = ["serde"] }' >> Cargo.toml
fi

if ! grep -q "tracing" Cargo.toml; then
    echo 'tracing = "0.1"' >> Cargo.toml
fi

echo "✅ All files created and dependencies added!"

# 6. Set environment variable and test build
echo "🧪 Testing build..."
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"

if cargo build; then
    echo "🎉 Backend builds successfully!"
    echo ""
    echo "Next steps:"
    echo "1. Run 'export DATABASE_URL=\"postgresql://postgres:postgres@localhost:5432/solana_wallet\"'"
    echo "2. Run 'cargo run' to start the server"
    echo "3. Test the endpoints:"
    echo "   - POST http://localhost:8080/api/user/signup"
    echo "   - POST http://localhost:8080/api/user/signin"
    echo "   - GET http://localhost:8080/api/solana/balance (requires auth)"
    echo "   - GET http://localhost:8080/health"
else
    echo "❌ Build failed. Check the errors above."
fi