use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use uuid::Uuid;
use actix_web::HttpRequest;
use actix_web::HttpMessage;
use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Registry};
use lazy_static::lazy_static;

use crate::AppState;
use crate::middleware::auth::Claims;

lazy_static! {
    pub static ref SOLANA_API_METRICS: SolanaApiMetrics = SolanaApiMetrics::new();
}

pub struct SolanaApiMetrics {
    pub transaction_count: IntCounterVec,
    pub transaction_latency: HistogramVec,
}

impl SolanaApiMetrics {
    pub fn new() -> Self {
        let transaction_count = IntCounterVec::new(
            prometheus::opts!("solana_transactions_total", "Total number of Solana transactions"),
            &["type", "status"]
        ).unwrap();
        
        let transaction_latency = HistogramVec::new(
            HistogramOpts::new(
                "solana_transaction_duration_seconds",
                "Histogram of Solana transaction durations in seconds"
            ),
            &["type"]
        ).unwrap();
        
        Self {
            transaction_count,
            transaction_latency,
        }
    }
    
    pub fn register(&self, registry: &Registry) -> Result<(), prometheus::Error> {
        registry.register(Box::new(self.transaction_count.clone()))?;
        registry.register(Box::new(self.transaction_latency.clone()))?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AddressRequest {
    pub public_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddressResponse {
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct TransferRequest {
    pub to_address: String,
    pub lamports: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TransferResponse {
    pub transaction_signature: String,
    pub status: String,
}

/// Derive Solana address from user's public key
pub async fn derive_address(
    _req: HttpRequest,
    data: web::Data<AppState>,
    address_req: web::Json<AddressRequest>
) -> impl Responder {
    // Validate public key format
    if address_req.public_key.len() != 64 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Public key must be 64 hex characters (32 bytes)".to_string()),
        });
    }
    
    // Derive Solana address from public key
    let solana_address = match data.solana_blockchain.derive_solana_address(&address_req.public_key) {
        Ok(address) => address,
        Err(e) => {
            error!("Failed to derive Solana address: {}", e);
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(format!("Invalid public key format: {}", e)),
            });
        }
    };
    
    // Return successful response
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(AddressResponse {
            address: solana_address,
        }),
        error: None,
    })
}

/// Transfer SOL to another address
pub async fn transfer_sol(
    req: HttpRequest,
    data: web::Data<AppState>,
    transfer_req: web::Json<TransferRequest>
) -> impl Responder {
    let timer = SOLANA_API_METRICS.transaction_latency.with_label_values(&["transfer"]).start_timer();
    
    // Extract user ID from claims
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => claims.sub.clone(),
        None => {
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "auth_error"]).inc();
            return HttpResponse::Unauthorized().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Authentication required".to_string()),
            });
        }
    };
    
    // Validate recipient address
    if !data.solana_blockchain.validate_address(&transfer_req.to_address) {
        SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "validation_error"]).inc();
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Invalid recipient address".to_string()),
        });
    }
    
    info!("Transferring {} lamports to {} for user {}", transfer_req.lamports, transfer_req.to_address, user_id);
    
    // Get user's public key
    let user_id_uuid = match Uuid::parse_str(&user_id) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid UUID format: {}", e);
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "uuid_error"]).inc();
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid user ID format".to_string()),
            });
        }
    };
    
    let user = match data.store.get_user_by_id(&user_id_uuid).await {
        Ok(user) => user,
        Err(e) => {
            error!("Failed to get user: {}", e);
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "database_error"]).inc();
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Failed to retrieve user information".to_string()),
            });
        }
    };
    
    let user_public_key = match user.public_key.as_ref() {
        Some(pk) => pk.clone(),
        None => {
            error!("User has no public key: {}", user_id);
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "wallet_error"]).inc();
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Wallet not initialized. Please generate MPC keys first.".to_string()),
            });
        }
    };
    
    // Get Solana address from public key
    let solana_address = match data.solana_blockchain.derive_solana_address(&user_public_key) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to derive Solana address: {}", e);
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "address_error"]).inc();
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Failed to derive Solana address".to_string()),
            });
        }
    };
    
    // Build SOL transfer transaction
    let unsigned_tx = match data.solana_blockchain.build_sol_transfer(
        &solana_address, 
        &transfer_req.to_address, 
        transfer_req.lamports
    ).await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to build SOL transfer: {}", e);
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "build_error"]).inc();
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(format!("Transaction build failed: {}", e)),
            });
        }
    };
    
    // Sign transaction with MPC
    let mpc_signature = match data.mpc_client.sign_transaction(
        &user_id.to_string(), 
        &unsigned_tx.message_hash,
        &unsigned_tx.transaction_data
    ).await {
        Ok(sig) => sig,
        Err(e) => {
            warn!("MPC signing failed: {} - this is expected until MPC is fully implemented", e);
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "signing_error"]).inc();
            return HttpResponse::ServiceUnavailable().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Transaction signing service unavailable".to_string()),
            });
        }
    };
    
    // Broadcast transaction
    let signature = match data.solana_blockchain.broadcast_transaction(
        &unsigned_tx.transaction_data, 
        &mpc_signature
    ).await {
        Ok(sig) => sig,
        Err(e) => {
            error!("Transaction broadcast failed: {}", e);
            SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "broadcast_error"]).inc();
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(format!("Failed to broadcast transaction: {}", e)),
            });
        }
    };
    
    info!("SOL transfer successful for user {}: {}", user_id, signature);
    SOLANA_API_METRICS.transaction_count.with_label_values(&["transfer", "success"]).inc();
    
    // Record transaction latency
    timer.observe_duration();
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(TransferResponse {
            transaction_signature: signature,
            status: "pending".to_string(),
        }),
        error: None,
    })
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/solana")
            .route("/address", web::post().to(derive_address))
            .route("/transfer", web::post().to(transfer_sol))
    );
}