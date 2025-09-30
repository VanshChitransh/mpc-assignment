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
