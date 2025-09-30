// Simple Solana API server for testing Phase 4.1 implementation
use actix_web::{web, App, HttpServer, HttpResponse, Result, middleware::Logger};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize)]
struct DeriveAddressRequest {
    pub public_key: String,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Serialize)]
struct DeriveAddressResponse {
    pub address: String,
}

#[derive(Deserialize)]
struct TransferRequest {
    pub to_address: String,
    pub lamports: u64,
}

#[derive(Serialize)]
struct TransferResponse {
    pub transaction_signature: String,
    pub status: String,
}

// Simulate our Solana blockchain functions
fn derive_solana_address(public_key: &str) -> Result<String, String> {
    if public_key.len() != 64 {
        return Err("Invalid public key length: expected 64 characters".to_string());
    }
    
    let pubkey_bytes = hex::decode(public_key)
        .map_err(|e| format!("Invalid hex public key: {}", e))?;
    
    if pubkey_bytes.len() != 32 {
        return Err("Invalid public key: expected 32 bytes".to_string());
    }
    
    let address = bs58::encode(&pubkey_bytes).into_string();
    Ok(address)
}

fn validate_address(address: &str) -> bool {
    if address.len() < 32 || address.len() > 44 {
        return false;
    }
    
    bs58::decode(address).into_vec().is_ok()
}

async fn derive_address(req: web::Json<DeriveAddressRequest>) -> Result<HttpResponse> {
    println!("Deriving address for public key: {}...", &req.public_key[..8]);
    
    match derive_solana_address(&req.public_key) {
        Ok(address) => {
            println!("Successfully derived address: {}", address);
            let response = ApiResponse {
                success: true,
                data: Some(DeriveAddressResponse { address }),
                error: None,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            println!("Address derivation failed: {}", e);
            let response = ApiResponse::<DeriveAddressResponse> {
                success: false,
                data: None,
                error: Some(e),
            };
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

async fn transfer(req: web::Json<TransferRequest>) -> Result<HttpResponse> {
    println!("Transfer request: {} lamports to {}", req.lamports, req.to_address);
    
    // Validate recipient address
    if !validate_address(&req.to_address) {
        let response = ApiResponse::<TransferResponse> {
            success: false,
            data: None,
            error: Some("Invalid recipient address format".to_string()),
        };
        return Ok(HttpResponse::BadRequest().json(response));
    }
    
    // Simulate transaction processing
    let mock_signature = format!("mock_signature_{}", chrono::Utc::now().timestamp());
    
    println!("Simulated transaction signature: {}", mock_signature);
    
    let response = ApiResponse {
        success: true,
        data: Some(TransferResponse {
            transaction_signature: mock_signature,
            status: "pending".to_string(),
        }),
        error: None,
    };
    Ok(HttpResponse::Ok().json(response))
}

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "solana-integration-test",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("127.0.0.1:{}", port);
    
    println!("Starting Solana Integration Test Server on {}", bind_address);
    println!("Available endpoints:");
    println!("  POST /api/v1/solana/address");
    println!("  POST /api/v1/solana/transfer");
    println!("  GET  /health");
    
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .service(
                web::scope("/api/v1/solana")
                    .route("/address", web::post().to(derive_address))
                    .route("/transfer", web::post().to(transfer))
            )
    })
    .bind(bind_address)?
    .run()
    .await
}
