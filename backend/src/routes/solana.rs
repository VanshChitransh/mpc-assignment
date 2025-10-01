use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use uuid::Uuid;
use actix_web::{HttpRequest, HttpMessage};
use std::str::FromStr;

use crate::AppState;
use crate::middleware::auth::get_user_id;

#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
    pub success: bool,
    pub balances: Vec<TokenBalance>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub symbol: String,
    pub decimals: i32,
    pub amount: String,
    pub ui_amount: f64,
    pub logo_uri: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: String, // String to handle large numbers safely
    pub slippage: Option<f64>, // In percentage, e.g. 0.5 for 0.5%
}

#[derive(Serialize, Deserialize)]
pub struct QuoteResponse {
    pub success: bool,
    pub quote_id: Option<String>,
    pub in_amount: Option<String>,
    pub out_amount: Option<String>,
    pub price_impact: Option<f64>,
    pub platform_fee: Option<f64>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SwapRequest {
    pub quote_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct SwapResponse {
    pub success: bool,
    pub transaction_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SendRequest {
    pub to_address: String,
    pub mint: String,         // Native SOL is "So11111111111111111111111111111111111111112"
    pub amount: String,       // String to handle large numbers safely
    pub decimals: Option<i32>, // Needed for UI amount conversion
}

#[derive(Serialize, Deserialize)]
pub struct SendResponse {
    pub success: bool,
    pub transaction_id: String,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

/// Get balances for authenticated user
pub async fn get_balances(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    // Extract user ID
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(err_response) => return err_response,
    };
    
    info!("Getting balances for user {}", user_id);
    
    // Get user information
    let user = match data.store.get_user_by_id(&user_id).await {
        Ok(user) => user,
        Err(e) => {
            error!("Failed to get user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve user information".to_string(),
            });
        }
    };
    
    // Check if user has a public key
    let public_key = match user.public_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "User has no wallet. Please generate MPC keys first.".to_string(),
            });
        }
    };
    
    // Derive Solana address
    let solana_address = match data.solana_blockchain.derive_solana_address(&public_key) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to derive Solana address: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to derive wallet address".to_string(),
            });
        }
    };
    
    // Get SOL balance
    let sol_balance = match data.solana_blockchain.get_sol_balance(&solana_address).await {
        Ok(balance) => balance,
        Err(e) => {
            error!("Failed to get SOL balance: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to fetch SOL balance".to_string(),
            });
        }
    };
    
    // Get token balances
    let token_balances_result = data.solana_blockchain.get_token_balances(&solana_address).await;
    
    // Prepare response balances
    let mut balances = Vec::new();
    
    // Add SOL balance
    balances.push(TokenBalance {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "SOL".to_string(),
        decimals: 9,
        amount: sol_balance.lamports.to_string(),
        ui_amount: sol_balance.ui_balance,
        logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png".to_string()),
    });
    
    // Add token balances
    if let Ok(token_balances) = token_balances_result {
        for token_balance in token_balances {
            // Get token info from store
            let token_info = data.store.get_asset_by_mint(&token_balance.mint).await;
            
            let symbol = match &token_info {
                Ok(asset) => asset.symbol.clone(),
                Err(_) => "Unknown".to_string(),
            };
            
            let logo_uri = match &token_info {
                Ok(asset) => asset.logo_url.clone(),
                Err(_) => None,
            };
            
            balances.push(TokenBalance {
                mint: token_balance.mint,
                symbol,
                decimals: token_balance.decimals as i32,
                amount: token_balance.amount,
                ui_amount: token_balance.ui_amount,
                logo_uri,
            });
        }
    }
    
    HttpResponse::Ok().json(BalanceResponse {
        success: true,
        balances,
        message: None,
    })
}

/// Get quote for token swap
pub async fn get_quote(
    req: HttpRequest, 
    data: web::Data<AppState>, 
    quote_req: web::Json<QuoteRequest>
) -> impl Responder {
    // Extract user ID
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(err_response) => return err_response,
    };
    
    info!(
        "Getting swap quote for user {} - {} to {}, amount: {}", 
        user_id, 
        quote_req.input_mint, 
        quote_req.output_mint, 
        quote_req.amount
    );
    
    // Parse amount
    let amount = match quote_req.amount.parse::<u64>() {
        Ok(a) => a,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid amount format".to_string(),
            });
        }
    };
    
    // TODO: Replace with actual Jupiter API integration
    // For now, we return a mock response
    let quote_id = Uuid::new_v4();
    
    // Store the quote in the database
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(2);
    
    let quote_data = serde_json::json!({
        "inputMint": quote_req.input_mint,
        "outputMint": quote_req.output_mint,
        "amount": quote_req.amount,
        "slippage": quote_req.slippage.unwrap_or(0.5),
        "outAmount": (amount / 10).to_string(),
        "priceImpact": 0.1,
        "platformFee": 0.0,
        "routeInfo": {
            "marketInfos": [
                {
                    "id": "mock-market",
                    "label": "Mock Market",
                    "inputMint": quote_req.input_mint,
                    "outputMint": quote_req.output_mint,
                    "liquidityFee": 0.0
                }
            ]
        }
    });
    
    match data.store.create_quote(
        &user_id,
        &quote_req.input_mint,
        &quote_req.output_mint,
        amount,
        amount / 10, // Mock output amount
        quote_data,
        expires_at
    ).await {
        Ok(saved_quote) => {
            HttpResponse::Ok().json(QuoteResponse {
                success: true,
                quote_id: Some(saved_quote.id.to_string()),
                in_amount: Some(quote_req.amount.clone()),
                out_amount: Some((amount / 10).to_string()),
                price_impact: Some(0.1), // 0.1%
                platform_fee: Some(0.0), // No fee for now
                error: None,
            })
        },
        Err(e) => {
            error!("Failed to save quote: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to save quote".to_string(),
            })
        }
    }
}

/// Execute token swap
pub async fn execute_swap(
    req: HttpRequest, 
    data: web::Data<AppState>, 
    swap_req: web::Json<SwapRequest>
) -> impl Responder {
    // Extract user ID
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(err_response) => return err_response,
    };
    
    info!("Executing swap for user {} with quote {}", user_id, swap_req.quote_id);
    
    // Parse quote ID
    let quote_id = match Uuid::from_str(&swap_req.quote_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid quote ID format".to_string(),
            });
        }
    };
    
    // Get the quote from database
    let quote = match data.store.get_valid_quote(&quote_id, &user_id).await {
        Ok(q) => q,
        Err(e) => {
            error!("Failed to get quote: {}", e);
            return HttpResponse::NotFound().json(ErrorResponse {
                success: false,
                error: "Quote not found or expired".to_string(),
            });
        }
    };
    
    // Get user information
    let user = match data.store.get_user_by_id(&user_id).await {
        Ok(user) => user,
        Err(e) => {
            error!("Failed to get user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve user information".to_string(),
            });
        }
    };
    
    // Check if user has a public key
    let public_key = match user.public_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "User has no wallet. Please generate MPC keys first.".to_string(),
            });
        }
    };
    
    // TODO: Replace with actual Jupiter swap transaction building
    // For now, we just return a mock transaction ID
    let tx_signature = format!("mock_swap_tx_{}", Uuid::new_v4());
    
    // Mark quote as used
    match data.store.mark_quote_used(&quote_id).await {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to mark quote as used: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to update quote status".to_string(),
            });
        }
    };
    
    HttpResponse::Ok().json(SwapResponse {
        success: true,
        transaction_id: Some(tx_signature),
        error: None,
    })
}

/// Send tokens to an address
pub async fn send_tokens(
    req: HttpRequest,
    data: web::Data<AppState>,
    send_req: web::Json<SendRequest>
) -> impl Responder {
    // Extract user ID
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(err_response) => return err_response,
    };
    
    // Validate recipient address
    if !data.solana_blockchain.validate_address(&send_req.to_address) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            success: false,
            error: "Invalid recipient address".to_string(),
        });
    }
    
    // Parse amount
    let amount = match send_req.amount.parse::<u64>() {
        Ok(a) => a,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid amount format".to_string(),
            });
        }
    };
    
    info!("Sending {} tokens to {} for user {}", amount, send_req.to_address, user_id);
    
    // Get user details from database
    let user = match data.store.get_user_by_id(&user_id).await {
        Ok(user) => user,
        Err(e) => {
            error!("Failed to get user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to retrieve user information".to_string(),
            });
        }
    };
    
    let user_public_key = match user.public_key {
        Some(pk) => pk,
        None => {
            error!("User has no public key: {}", user_id);
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Wallet not initialized. Please generate MPC keys first.".to_string(),
            });
        }
    };
    
    // Get Solana address from public key
    let solana_address = match data.solana_blockchain.derive_solana_address(&user_public_key) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to derive Solana address: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: "Failed to derive Solana address".to_string(),
            });
        }
    };
    
    // Build transaction
    let unsigned_tx = if send_req.mint == "So11111111111111111111111111111111111111112" {
        // SOL transfer
        match data.solana_blockchain.build_sol_transfer(&solana_address, &send_req.to_address, amount).await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to build SOL transfer: {}", e);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    success: false,
                    error: format!("Transaction build failed: {}", e),
                });
            }
        }
    } else {
        // SPL token transfer
        match data.solana_blockchain.build_token_transfer(&solana_address, &send_req.to_address, &send_req.mint, amount).await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to build token transfer: {}", e);
                return HttpResponse::BadRequest().json(ErrorResponse {
                    success: false,
                    error: format!("Transaction build failed: {}", e),
                });
            }
        }
    };
    
    // Sign transaction with MPC
    let mpc_signature = match data.mpc_client
        .sign_transaction(&user_id.to_string(), &unsigned_tx.message_hash, &unsigned_tx.transaction_data)
        .await 
    {
        Ok(sig) => sig,
        Err(e) => {
            error!("MPC signing failed: {}", e);
            return HttpResponse::ServiceUnavailable().json(ErrorResponse {
                success: false,
                error: format!("Transaction signing service unavailable: {}", e),
            });
        }
    };
    
    // Broadcast transaction
    let signature = match data.solana_blockchain
        .broadcast_transaction(&unsigned_tx.transaction_data, &mpc_signature)
        .await
    {
        Ok(sig) => sig,
        Err(e) => {
            error!("Transaction broadcast failed: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to broadcast transaction: {}", e),
            });
        }
    };
    
    info!("Send transaction successful for user {}: {}", user_id, signature);
    
    HttpResponse::Ok().json(SendResponse {
        success: true,
        transaction_id: signature,
        message: "Transfer completed successfully".to_string(),
    })
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/solana")
            .route("/balance", web::get().to(get_balances))
            .route("/quote", web::post().to(get_quote))
            .route("/swap", web::post().to(execute_swap))
            .route("/send", web::post().to(send_tokens))
    );
}