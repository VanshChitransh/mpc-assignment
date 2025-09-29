mod error;
mod tss;
mod serialization;

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Result, middleware::Logger};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;
use crate::tss::ThresholdSigningService;

#[derive(Debug, Clone)]
pub struct AppState {
    pub tss: Arc<ThresholdSigningService>,
    pub node_id: u32,
    pub peer_nodes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyGenRequest {
    pub user_id: String,
    pub threshold: u32,
    pub total_parties: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyGenResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateKeysRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateKeysResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignRequest {
    pub user_id: String,
    pub message_hash: String,
    pub transaction_data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignResponse {
    pub success: bool,
    pub signature: Option<String>,
    pub error: Option<String>,
}

// Health check endpoint
async fn health(data: web::Data<AppState>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "node_id": data.node_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

// Key generation endpoint - matches test script's /generate
async fn generate_key(
    data: web::Data<AppState>,
    req: web::Json<KeyGenRequest>,
) -> Result<HttpResponse> {
    let request = req.into_inner();
    info!("Received key generation request for user: {}", request.user_id);

    let user_id = match Uuid::parse_str(&request.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(KeyGenResponse {
                success: false,
                public_key: None,
                error: Some("Invalid user ID format".to_string()),
            }));
        }
    };

    match data.tss.generate_key_share(&user_id, request.threshold as u16, request.total_parties as u16).await {
        Ok(()) => {
            // Get the generated public key
            match data.tss.get_public_key(&user_id).await {
                Ok(Some(public_key)) => {
                    info!("Key generation successful for user: {}", request.user_id);
                    Ok(HttpResponse::Ok().json(KeyGenResponse {
                        success: true,
                        public_key: Some(public_key),
                        error: None,
                    }))
                }
                Ok(None) => {
                    Ok(HttpResponse::InternalServerError().json(KeyGenResponse {
                        success: false,
                        public_key: None,
                        error: Some("Key generation succeeded but public key not found".to_string()),
                    }))
                }
                Err(e) => {
                    Ok(HttpResponse::InternalServerError().json(KeyGenResponse {
                        success: false,
                        public_key: None,
                        error: Some(e.to_string()),
                    }))
                }
            }
        }
        Err(e) => {
            error!("Key generation failed for user {}: {}", request.user_id, e);
            Ok(HttpResponse::InternalServerError().json(KeyGenResponse {
                success: false,
                public_key: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

// Aggregate keys endpoint - matches test script's /aggregate-keys
async fn aggregate_keys(
    data: web::Data<AppState>,
    req: web::Json<AggregateKeysRequest>,
) -> Result<HttpResponse> {
    let request = req.into_inner();
    info!("Received aggregate keys request for user: {}", request.user_id);

    let user_id = match Uuid::parse_str(&request.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(AggregateKeysResponse {
                success: false,
                public_key: None,
                error: Some("Invalid user ID format".to_string()),
            }));
        }
    };

    match data.tss.get_public_key(&user_id).await {
        Ok(Some(public_key)) => {
            info!("Public key retrieved for user: {}", request.user_id);
            Ok(HttpResponse::Ok().json(AggregateKeysResponse {
                success: true,
                public_key: Some(public_key),
                error: None,
            }))
        }
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(AggregateKeysResponse {
                success: false,
                public_key: None,
                error: Some("No key found for user".to_string()),
            }))
        }
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(AggregateKeysResponse {
                success: false,
                public_key: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

// Signing phase 1 endpoint - matches test script's /agg-send-step1
async fn sign_step1(
    data: web::Data<AppState>,
    req: web::Json<SignRequest>,
) -> Result<HttpResponse> {
    let request = req.into_inner();
    info!("Received signing step 1 request for user: {}", request.user_id);

    let user_id = match Uuid::parse_str(&request.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(SignResponse {
                success: false,
                signature: None,
                error: Some("Invalid user ID format".to_string()),
            }));
        }
    };

    // Simplified: just prepare signing session
    match data.tss.prepare_signing(&user_id, &request.message_hash).await {
        Ok(_) => {
            info!("Signing step 1 successful for user: {}", request.user_id);
            Ok(HttpResponse::Ok().json(SignResponse {
                success: true,
                signature: None,
                error: None,
            }))
        }
        Err(e) => {
            error!("Signing step 1 failed for user {}: {}", request.user_id, e);
            Ok(HttpResponse::InternalServerError().json(SignResponse {
                success: false,
                signature: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

// Signing phase 2 endpoint - matches test script's /agg-send-step2
async fn sign_step2(
    data: web::Data<AppState>,
    req: web::Json<SignRequest>,
) -> Result<HttpResponse> {
    let request = req.into_inner();
    info!("Received signing step 2 request for user: {}", request.user_id);

    let user_id = match Uuid::parse_str(&request.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(SignResponse {
                success: false,
                signature: None,
                error: Some("Invalid user ID format".to_string()),
            }));
        }
    };

    // Simplified: generate signature
    match data.tss.sign_message(&user_id, &request.message_hash).await {
        Ok(signature) => {
            info!("Signing step 2 successful for user: {}", request.user_id);
            Ok(HttpResponse::Ok().json(SignResponse {
                success: true,
                signature: Some(signature),
                error: None,
            }))
        }
        Err(e) => {
            error!("Signing step 2 failed for user {}: {}", request.user_id, e);
            Ok(HttpResponse::InternalServerError().json(SignResponse {
                success: false,
                signature: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    fmt()
        .with_env_filter(EnvFilter::new("mpc=info"))
        .init();

    info!("Starting MPC node...");

    // Get configuration from environment
    let node_id: u32 = env::var("NODE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .expect("NODE_ID must be a valid number");

    let bind_address = env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| format!("127.0.0.1:800{}", node_id));

    let data_dir = env::var("DATA_DIR")
        .unwrap_or_else(|_| format!("./data/node{}", node_id));

    // Parse peer nodes from environment
    let peer_nodes = env::var("PEER_NODES")
        .map(|nodes| nodes.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|_| vec![
            "http://localhost:8001".to_string(),
            "http://localhost:8002".to_string(), 
            "http://localhost:8003".to_string(),
        ]);

    info!("Node ID: {}", node_id);
    info!("Bind address: {}", bind_address);
    info!("Data directory: {}", data_dir);

    // Create data directory
    std::fs::create_dir_all(&data_dir)
        .expect("Failed to create data directory");

    // Initialize threshold signing service
    let tss = match ThresholdSigningService::new(node_id, &data_dir, peer_nodes.clone()).await {
        Ok(tss) => Arc::new(tss),
        Err(e) => {
            error!("Failed to initialize TSS: {}", e);
            std::process::exit(1);
        }
    };

    // Create application state
    let app_state = web::Data::new(AppState {
        tss,
        node_id,
        peer_nodes,
    });

    info!("MPC node {} starting on {}", node_id, bind_address);

    // Start HTTP server with test script compatible endpoints
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(Logger::default())
            // Health endpoint
            .route("/health", web::get().to(health))
            // Test script compatible endpoints
            .route("/generate", web::post().to(generate_key))
            .route("/aggregate-keys", web::post().to(aggregate_keys))
            .route("/agg-send-step1", web::post().to(sign_step1))
            .route("/agg-send-step2", web::post().to(sign_step2))
            // Also support API prefix versions
            .route("/api/keygen", web::post().to(generate_key))
            .route("/api/aggregate", web::post().to(aggregate_keys))
            .route("/api/sign-phase1", web::post().to(sign_step1))
            .route("/api/sign-phase2", web::post().to(sign_step2))
    })
    .bind(bind_address)?
    .run()
    .await
}
