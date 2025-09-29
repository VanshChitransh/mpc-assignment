use crate::middleware::AuthExtensions;
use crate::services::{JupiterClient, JupiterError, SolanaClient};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use store::Store;  // Removed unused BalanceError import
use tracing::{error, info, warn};
use uuid::Uuid;

// Balance endpoint structures
#[derive(Serialize)]
pub struct BalanceResponse {
    pub success: bool,
    pub balances: Vec<TokenBalance>,
}

#[derive(Serialize)]
pub struct TokenBalance {
    pub mint: String,
    pub symbol: String,
    pub balance: String,
    pub decimals: u8,
    pub usd_value: Option<f64>,
}

// Quote endpoint structures
#[derive(Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: String,
}

#[derive(Serialize)]
pub struct QuoteResponse {
    pub success: bool,
    pub quote_id: String,
    pub out_amount: String,
    pub price_impact_pct: f64,
}

// Swap endpoint structures
#[derive(Deserialize)]
pub struct SwapRequest {
    pub quote_id: String,
}

#[derive(Serialize)]
pub struct SwapResponse {
    pub success: bool,
    pub transaction_id: String,
    pub message: String,
}

// Send endpoint structures
#[derive(Deserialize)]
pub struct SendRequest {
    pub to_address: String,
    pub mint: String,
    pub amount: String,
}

#[derive(Serialize)]
pub struct SendResponse {
    pub success: bool,
    pub transaction_id: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

pub async fn get_balance(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            error!("No user ID found in authenticated request");
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };
    
    info!("Getting balances for user ID: {}", user_id);
    
    let store = Store::new(data.db.clone());
    
    // Get SOL balance
    let sol_balance = match store.get_sol_balance(&user_id).await {
        Ok(balance) => balance,
        Err(e) => {
            error!("Failed to get SOL balance for user {}: {}", user_id, e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve SOL balance".to_string(),
            }));
        }
    };
    
    // Get token balances
    let token_balances = match store.get_token_balances(&user_id).await {
        Ok(balances) => balances,
        Err(e) => {
            error!("Failed to get token balances for user {}: {}", user_id, e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve token balances".to_string(),
            }));
        }
    };
    
    // Combine SOL and token balances
    let mut all_balances = vec![TokenBalance {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "SOL".to_string(),
        balance: sol_balance.to_string(),
        decimals: 9,
        usd_value: None,
    }];
    
    for token_balance in token_balances {
        all_balances.push(TokenBalance {
            mint: token_balance.token_mint,
            symbol: token_balance.symbol,
            balance: token_balance.balance.to_string(),
            decimals: token_balance.decimals as u8,
            usd_value: None,
        });
    }
    
    info!("Successfully retrieved {} balances for user: {}", all_balances.len(), user_id);
    
    Ok(HttpResponse::Ok().json(BalanceResponse {
        success: true,
        balances: all_balances,
    }))
}

pub async fn get_quote(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<QuoteRequest>,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            error!("No user ID found in authenticated request");
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };
    
    let quote_req = req_body.into_inner();
    info!("Getting quote for user {}: {} {} -> {}", 
          user_id, quote_req.amount, quote_req.input_mint, quote_req.output_mint);
    
    // Parse amount
    let amount = match quote_req.amount.parse::<u64>() {
        Ok(amt) => amt,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid amount format".to_string(),
            }));
        }
    };
    
    // Get Jupiter quote
    let jupiter_client = &data.jupiter_client;
    let jupiter_quote = match jupiter_client.get_quote(
        &quote_req.input_mint,
        &quote_req.output_mint,
        amount,
        Some(50), // 0.5% slippage
    ).await {
        Ok(quote) => quote,
        Err(JupiterError::NoRoutesFound) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "No swap route found for this token pair".to_string(),
            }));
        }
        Err(e) => {
            error!("Jupiter quote failed: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(ErrorResponse {
                success: false,
                error: "Quote service temporarily unavailable".to_string(),
            }));
        }
    };
    
    // Parse amounts and price impact using static method calls
    let out_amount = match JupiterClient::parse_amount(&jupiter_quote.out_amount) {
        Ok(amt) => amt,
        Err(_) => {
            error!("Failed to parse Jupiter out_amount: {}", jupiter_quote.out_amount);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Invalid quote response from Jupiter".to_string(),
            }));
        }
    };
    
    let price_impact = JupiterClient::parse_price_impact(&jupiter_quote.price_impact_pct);
    
    // Store quote in database
    let store = Store::new(data.db.clone());
    let quote_data = serde_json::to_value(&jupiter_quote).unwrap_or_else(|_| serde_json::json!({}));
    
    let stored_quote = match store.store_quote(
        &user_id,
        &quote_req.input_mint,
        &quote_req.output_mint,
        amount as i64,
        out_amount as i64,
        quote_data,
        300, // 5 minutes expiry
    ).await {
        Ok(quote) => quote,
        Err(e) => {
            error!("Failed to store quote: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to store quote".to_string(),
            }));
        }
    };
    
    info!("Successfully created quote {} for user {}", stored_quote.id, user_id);
    
    Ok(HttpResponse::Ok().json(QuoteResponse {
        success: true,
        quote_id: stored_quote.id.to_string(),
        out_amount: out_amount.to_string(),
        price_impact_pct: price_impact,
    }))
}

pub async fn execute_swap(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<SwapRequest>,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            error!("No user ID found in authenticated request");
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
    
    // Get and validate quote
    let store = Store::new(data.db.clone());
    let quote = match store.get_valid_quote(&quote_id, &user_id).await {
        Ok(quote) => quote,
        Err(store::QuoteError::QuoteNotFound) => {
            return Ok(HttpResponse::NotFound().json(ErrorResponse {
                success: false,
                error: "Quote not found".to_string(),
            }));
        }
        Err(store::QuoteError::QuoteExpired) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Quote has expired".to_string(),
            }));
        }
        Err(store::QuoteError::QuoteAlreadyUsed) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Quote has already been used".to_string(),
            }));
        }
        Err(e) => {
            error!("Failed to get quote: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Database error".to_string(),
            }));
        }
    };
    
    // Get user's public key
    let user_public_key = match store.get_user_public_key(&user_id).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "User wallet not initialized".to_string(),
            }));
        }
        Err(e) => {
            error!("Failed to get user public key: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve user wallet".to_string(),
            }));
        }
    };
    
    // Get swap transaction from Jupiter
    let jupiter_client = &data.jupiter_client;
    let swap_response = match jupiter_client.get_swap_transaction(
        quote.quote_data,
        &user_public_key,
    ).await {
        Ok(response) => response,
        Err(e) => {
            error!("Jupiter swap transaction failed: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(ErrorResponse {
                success: false,
                error: "Failed to create swap transaction".to_string(),
            }));
        }
    };
    
    // Sign transaction with MPC
    let transaction_hash = "placeholder_hash"; // TODO: Extract from transaction
    let mpc_signature = match data.mpc_client.sign_transaction(
        &user_id,
        transaction_hash,
        &swap_response.swap_transaction,
    ).await {
        Ok(sig) => sig,
        Err(e) => {
            warn!("MPC signing failed: {} - this is expected until MPC is fully implemented", e);
            return Ok(HttpResponse::ServiceUnavailable().json(ErrorResponse {
                success: false,
                error: "Transaction signing service unavailable".to_string(),
            }));
        }
    };
    
    // Broadcast transaction
    let solana_client = &data.solana_client;
    let signature = match solana_client.broadcast_transaction(
        &swap_response.swap_transaction,
        vec![mpc_signature],
    ).await {
        Ok(sig) => sig,
        Err(e) => {
            error!("Transaction broadcast failed: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to broadcast transaction".to_string(),
            }));
        }
    };
    
    // Mark quote as used
    if let Err(e) = store.mark_quote_used(&quote_id).await {
        warn!("Failed to mark quote as used: {}", e);
        // Don't fail the entire request since the swap succeeded
    }
    
    info!("Swap executed successfully for user {}: {}", user_id, signature);
    
    Ok(HttpResponse::Ok().json(SwapResponse {
        success: true,
        transaction_id: signature,
        message: "Swap executed successfully".to_string(),
    }))
}

pub async fn send_tokens(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<SendRequest>,
) -> Result<HttpResponse> {
    let user_id = match req.get_user_id() {
        Some(id) => id,
        None => {
            error!("No user ID found in authenticated request");
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                success: false,
                error: "Authentication required".to_string(),
            }));
        }
    };
    
    let send_req = req_body.into_inner();
    info!("Sending tokens for user {}: {} {} to {}", 
          user_id, send_req.amount, send_req.mint, send_req.to_address);
    
    // Validate recipient address
    if !SolanaClient::validate_address(&send_req.to_address) {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            success: false,
            error: "Invalid recipient address".to_string(),
        }));
    }
    
    // Parse amount
    let amount = match send_req.amount.parse::<u64>() {
        Ok(amt) => amt,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid amount format".to_string(),
            }));
        }
    };
    
    if amount == 0 {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            success: false,
            error: "Amount must be greater than zero".to_string(),
        }));
    }
    
    // Get user's public key
    let store = Store::new(data.db.clone());
    let user_public_key = match store.get_user_public_key(&user_id).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "User wallet not initialized".to_string(),
            }));
        }
        Err(e) => {
            error!("Failed to get user public key: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve user wallet".to_string(),
            }));
        }
    };
    
    // Build transaction
    let solana_client = &data.solana_client;
    let unsigned_tx = if send_req.mint == "So11111111111111111111111111111111111111112" {
        // SOL transfer
        match solana_client.build_sol_transfer(&user_public_key, &send_req.to_address, amount).await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to build SOL transfer: {}", e);
                return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                    success: false,
                    error: format!("Transaction build failed: {}", e),
                }));
            }
        }
    } else {
        // SPL token transfer
        match solana_client.build_token_transfer(&user_public_key, &send_req.to_address, &send_req.mint, amount).await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to build token transfer: {}", e);
                return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                    success: false,
                    error: format!("Transaction build failed: {}", e),
                }));
            }
        }
    };
    
    // Sign transaction with MPC
    let mpc_signature = match data.mpc_client.sign_transaction(
        &user_id,
        &unsigned_tx.message_hash,
        &unsigned_tx.transaction_data,
    ).await {
        Ok(sig) => sig,
        Err(e) => {
            warn!("MPC signing failed: {} - this is expected until MPC is fully implemented", e);
            return Ok(HttpResponse::ServiceUnavailable().json(ErrorResponse {
                success: false,
                error: "Transaction signing service unavailable".to_string(),
            }));
        }
    };
    
    // Broadcast transaction
    let signature = match solana_client.broadcast_transaction(
        &unsigned_tx.transaction_data,
        vec![mpc_signature],
    ).await {
        Ok(sig) => sig,
        Err(e) => {
            error!("Transaction broadcast failed: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to broadcast transaction".to_string(),
            }));
        }
    };
    
    info!("Send transaction successful for user {}: {}", user_id, signature);
    
    Ok(HttpResponse::Ok().json(SendResponse {
        success: true,
        transaction_id: signature,
        message: "Transfer completed successfully".to_string(),
    }))
}