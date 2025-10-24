use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use uuid::Uuid;

use crate::services::mpc::MpcClient;
use crate::services::jupiter::{JupiterClient, QuoteRequest as JupiterQuoteRequest};
use crate::blockchain::solana::{self, SolanaClient};
use crate::store::Store;

// Request/Response Types
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub success: bool,
    pub balances: Vec<TokenBalance>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub symbol: String,
    pub ui_amount: f64,
    pub decimals: i32,
    pub logo_uri: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: String,
    pub slippage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub success: bool,
    pub quote_id: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub price_impact: f64,
    pub expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapRequest {
    pub quote_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub success: bool,
    pub transaction_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendRequest {
    pub to_address: String,
    pub mint: String,
    pub amount: String,
    pub decimals: i32,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendResponse {
    pub success: bool,
    pub transaction_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// Get balances for the authenticated user
pub async fn get_balance(
    req: HttpRequest,
    store: web::Data<Store>,
    solana_client: web::Data<SolanaClient>,
) -> HttpResponse {
    // Extract user ID from request extensions (set by auth middleware)
    let user_id = match req.extensions().get::<String>() {
        Some(id) => id.to_string(),
        None => {
            return HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            });
        }
    };
    
    // Get user from database
    let user = match sqlx::query!(
        "SELECT id, email, public_key FROM users WHERE id = $1",
        Uuid::parse_str(&user_id).unwrap_or_default()
    )
    .fetch_one(&store.pool)
    .await {
        Ok(user) => user,
        Err(e) => {
            error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Internal server error".to_string(),
            });
        }
    };
    
    // Check if user has a public key
    let public_key = match user.public_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "No wallet key found for this user".to_string(),
            });
        }
    };
    
    // Get SOL balance (using mock implementation for now)
    let sol_balance = match solana_client.get_sol_balance(&public_key).await {
        Ok(balance) => balance,
        Err(e) => {
            error!("Error getting SOL balance: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve balances".to_string(),
            });
        }
    };
    
    // Prepare response with SOL balance
    let mut balances = vec![TokenBalance {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "SOL".to_string(),
        ui_amount: sol_balance.ui_amount,
        decimals: 9,
        logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png".to_string()),
    }];
    
    // Add some token balances
    let token_mints = [
        ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC", 6, "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png"),
        ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", "USDT", 6, "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.png"),
        ("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", "mSOL", 9, "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So/logo.png"),
    ];
    
    for (mint, symbol, decimals, logo) in token_mints.iter() {
        match solana_client.get_token_balance(&public_key, mint).await {
            Ok(balance) => {
                balances.push(TokenBalance {
                    mint: mint.to_string(),
                    symbol: symbol.to_string(),
                    ui_amount: balance.ui_amount,
                    decimals: *decimals,
                    logo_uri: Some(logo.to_string()),
                });
            },
            Err(_) => {
                // If balance retrieval fails, still include the token with 0 balance
                balances.push(TokenBalance {
                    mint: mint.to_string(),
                    symbol: symbol.to_string(),
                    ui_amount: 0.0,
                    decimals: *decimals,
                    logo_uri: Some(logo.to_string()),
                });
            }
        }
    }
    
    HttpResponse::Ok().json(BalanceResponse {
        success: true,
        balances,
        message: None,
    })
}

// Get quote for a token swap
pub async fn get_quote(
    req: HttpRequest,
    jupiter_client: web::Data<JupiterClient>,
    store: web::Data<Store>,
    quote_req: web::Json<QuoteRequest>,
) -> HttpResponse {
    // Extract user ID
    let user_id = match req.extensions().get::<String>() {
        Some(id) => id.to_string(),
        None => {
            return HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            });
        }
    };
    
    // Validate input parameters
    if quote_req.amount.parse::<f64>().is_err() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            success: false,
            error: "Invalid amount format".to_string(),
        });
    }
    
    // Convert to Jupiter quote request
    let jupiter_request = JupiterQuoteRequest {
        input_mint: quote_req.input_mint.clone(),
        output_mint: quote_req.output_mint.clone(),
        amount: quote_req.amount.clone(),
        slippage: quote_req.slippage,
    };
    
    // Get quote from Jupiter
    let jupiter_quote = match jupiter_client.get_quote(&jupiter_request).await {
        Ok(quote) => quote,
        Err(e) => {
            error!("Jupiter quote error: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to get swap quote".to_string(),
            });
        }
    };
    
    // Store quote in database (valid for 30 seconds)
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(30);
    let quote_data = match serde_json::to_value(&jupiter_quote) {
        Ok(data) => data,
        Err(e) => {
            error!("Serialization error: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Internal server error".to_string(),
            });
        }
    };
    
    // Parse amounts for database
    let in_amount = jupiter_quote.in_amount.parse::<u64>().unwrap_or(0);
    let out_amount = jupiter_quote.out_amount.parse::<u64>().unwrap_or(0);
    
    let quote_result = store.create_quote(
        &user_id,
        &quote_req.input_mint,
        &quote_req.output_mint,
        in_amount,
        out_amount,
        quote_data,
        expires_at,
    ).await;
    
    match quote_result {
        Ok(quote) => {
            // Return quote info to the client
            HttpResponse::Ok().json(QuoteResponse {
                success: true,
                quote_id: quote.id.to_string(),
                input_mint: quote_req.input_mint.clone(),
                output_mint: quote_req.output_mint.clone(),
                in_amount: jupiter_quote.in_amount.clone(),
                out_amount: jupiter_quote.out_amount.clone(),
                price_impact: jupiter_quote.price_impact_pct,
                expires_in: 30,
            })
        },
        Err(e) => {
            error!("Failed to store quote: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to store quote".to_string(),
            })
        }
    }
}

// Execute a token swap based on a previously obtained quote
pub async fn swap(
    req: HttpRequest,
    store: web::Data<Store>,
    _mpc_client: web::Data<MpcClient>,
    _solana_client: web::Data<SolanaClient>,
    swap_req: web::Json<SwapRequest>,
) -> HttpResponse {
    // Extract user ID
    let user_id = match req.extensions().get::<String>() {
        Some(id) => id.to_string(),
        None => {
            return HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            });
        }
    };
    
    // Parse quote ID
    let quote_id = match Uuid::parse_str(&swap_req.quote_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid quote ID".to_string(),
            });
        }
    };
    
    // Get quote from database
    let _quote = match store.get_valid_quote(&quote_id, &user_id).await {
        Ok(q) => q,
        Err(e) => {
            error!("Failed to get quote: {}", e);
            return HttpResponse::NotFound().json(ErrorResponse {
                success: false,
                error: "Quote not found, expired, or already used".to_string(),
            });
        }
    };
    
    // Get user from database to get their public key
    let user = match store.get_user_by_id(&user_id).await {
        Ok(user) => user,
        Err(e) => {
            error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Internal server error".to_string(),
            });
        }
    };
    
    // Check if user has a public key
    let _public_key = match user.public_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "No wallet key found for this user".to_string(),
            });
        }
    };
    
    // In a real implementation, we would:
    // 1. Get the swap transaction from Jupiter
    // 2. Extract the transaction message for signing
    // 3. Sign with MPC
    // 4. Broadcast the transaction
    
    // For now, just return a mock transaction ID
    let transaction_id = format!("mock_swap_{}", Uuid::new_v4());
    
    // Mark quote as used
    match store.mark_quote_used(&quote_id).await {
        Ok(_) => {},
        Err(e) => {
            error!("Failed to mark quote as used: {}", e);
        }
    }
    
    HttpResponse::Ok().json(SwapResponse {
        success: true,
        transaction_id: Some(transaction_id),
        message: Some("Swap executed successfully".to_string()),
    })
}

// Send tokens to a recipient address
pub async fn send(
    req: HttpRequest,
    store: web::Data<Store>,
    mpc_client: web::Data<MpcClient>,
    solana_client: web::Data<SolanaClient>,
    send_req: web::Json<SendRequest>,
) -> HttpResponse {
    // Extract user ID
    let user_id = match req.extensions().get::<String>() {
        Some(id) => id.to_string(),
        None => {
            return HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            });
        }
    };
    
    // Parse amount
    let amount = match send_req.amount.parse::<f64>() {
        Ok(a) => a,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid amount format".to_string(),
            });
        }
    };
    
    // Get user from database to get their public key
    let user = match store.get_user_by_id(&user_id).await {
        Ok(user) => user,
        Err(e) => {
            error!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Internal server error".to_string(),
            });
        }
    };
    
    // Check if user has a public key
    let public_key = match user.public_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "No wallet key found for this user".to_string(),
            });
        }
    };
    
    // Create transaction based on token type
    let transaction = if send_req.mint == "So11111111111111111111111111111111111111112" {
        // SOL transfer
        match solana_client.create_transfer_transaction(
            &public_key,
            &send_req.to_address,
            amount,
            send_req.memo.clone(),
        ).await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to create SOL transfer transaction: {:?}", e);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    success: false,
                    error: format!("Failed to create transaction: {:?}", e),
                });
            }
        }
    } else {
        // SPL token transfer
        match solana_client.create_token_transfer_transaction(
            &public_key,
            &send_req.to_address,
            &send_req.mint,
            amount,
            send_req.decimals as u8,
            send_req.memo.clone(),
        ).await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to create token transfer transaction: {:?}", e);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    success: false,
                    error: format!("Failed to create transaction: {:?}", e),
                });
            }
        }
    };
    
    // Extract transaction hash for signing
    let transaction_hash = solana::extract_transaction_hash(&transaction);
    
    // Sign with MPC (fixed to use 2 arguments)
    let signature = match mpc_client.sign_transaction(&user_id, &transaction_hash).await {
        Ok(sig) => sig,
        Err(e) => {
            error!("MPC signing failed: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Transaction signing failed".to_string(),
            });
        }
    };
    
    // Finalize the transaction with the signature
    let signed_transaction = solana::sign_and_finalize_transaction(transaction, signature);
    
    // Broadcast the transaction - Fixed async/await issue
    let result = match solana::broadcast_transaction("", signed_transaction).await {
        Ok(tx_id) => {
            info!("Transaction successfully sent: {}", tx_id);
            SendResponse {
                success: true,
                transaction_id: Some(tx_id),
                message: Some("Transaction sent successfully".to_string()),
            }
        },
        Err(e) => {
            error!("Failed to broadcast transaction: {:?}", e);
            SendResponse {
                success: false,
                transaction_id: None,
                message: Some(format!("Failed to broadcast transaction: {:?}", e)),
            }
        }
    };
    
    HttpResponse::Ok().json(result)
}