use actix_web::{web, App, HttpServer, HttpResponse, middleware::Logger};
use actix_cors::Cors;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tracing_subscriber;
use uuid::Uuid;

mod error;
mod serialization;
mod tss;

use error::MpcError;
use tss::ThresholdSigningService;

#[derive(Clone)]
pub struct AppState {
    pub tss: Arc<ThresholdSigningService>,
}

#[derive(Debug, Deserialize)]
struct KeyGenRequest {
    user_id: String,
    threshold: u16,
    total_parties: u16,
}

#[derive(Debug, Serialize)]
struct KeyGenResponse {
    success: bool,
    user_id: String,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct SignRound1Request {
    user_id: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct SignRound1Response {
    success: bool,
    commitments: String,
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct SignRound2Request {
    user_id: String,
    session_id: String,
    signing_package: String,
}

#[derive(Debug, Serialize)]
struct SignRound2Response {
    success: bool,
    signature_share: String,
}

#[derive(Debug, Deserialize)]
struct AggregateKeysRequest {
    user_id: String,
}

#[derive(Debug, Serialize)]
struct AggregateKeysResponse {
    success: bool,
    public_key: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AggregateSignatureRequest {
    signature_shares: Vec<String>,
    signing_package: String,
}

#[derive(Debug, Serialize)]
struct AggregateSignatureResponse {
    success: bool,
    signature: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "mpc-node"
    }))
}

async fn keygen(
    data: web::Data<AppState>,
    req: web::Json<KeyGenRequest>,
) -> HttpResponse {
    let user_id = match Uuid::parse_str(&req.user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid user ID format".to_string(),
            });
        }
    };

    match data.tss.generate_key_share(&user_id, req.threshold, req.total_parties).await {
        Ok(public_key) => HttpResponse::Ok().json(KeyGenResponse {
            success: true,
            user_id: user_id.to_string(),
            public_key,
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            success: false,
            error: format!("Key generation failed: {}", e),
        }),
    }
}

async fn aggregate_keys(
    data: web::Data<AppState>,
    req: web::Json<AggregateKeysRequest>,
) -> HttpResponse {
    // For now, just return the user's public key from storage
    let user_id = match Uuid::parse_str(&req.user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(AggregateKeysResponse {
                success: false,
                public_key: None,
                error: Some("Invalid user ID format".to_string()),
            });
        }
    };

    match data.tss.get_public_key(&user_id).await {
        Ok(Some(public_key)) => HttpResponse::Ok().json(AggregateKeysResponse {
            success: true,
            public_key: Some(public_key),
            error: None,
        }),
        Ok(None) => HttpResponse::NotFound().json(AggregateKeysResponse {
            success: false,
            public_key: None,
            error: Some("No key found for user".to_string()),
        }),
        Err(e) => HttpResponse::InternalServerError().json(AggregateKeysResponse {
            success: false,
            public_key: None,
            error: Some(format!("Failed to get public key: {}", e)),
        }),
    }
}

async fn sign_phase1(
    data: web::Data<AppState>,
    req: web::Json<SignRound1Request>,
) -> HttpResponse {
    let user_id = match Uuid::parse_str(&req.user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid user ID format".to_string(),
            });
        }
    };

    let message = match hex::decode(&req.message) {
        Ok(msg) => msg,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid message format (must be hex)".to_string(),
            });
        }
    };

    match data.tss.sign_round1(&user_id, &message).await {
        Ok((commitments, session_id)) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "nonce_commitment": hex::encode(&commitments),
            "signing_package": hex::encode(&session_id),
        })),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            success: false,
            error: format!("Phase 1 failed: {}", e),
        }),
    }
}

async fn sign_phase2(
    data: web::Data<AppState>,
    req: web::Json<SignRound2Request>,
) -> HttpResponse {
    let user_id = match Uuid::parse_str(&req.user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid user ID format".to_string(),
            });
        }
    };

    let signing_package = match hex::decode(&req.signing_package) {
        Ok(pkg) => pkg,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid signing package format".to_string(),
            });
        }
    };

    match data.tss.sign_round2(&user_id, &req.session_id, &signing_package).await {
        Ok(signature_share) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "signature_share": hex::encode(signature_share),
        })),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            success: false,
            error: format!("Phase 2 failed: {}", e),
        }),
    }
}

async fn aggregate_signature(
    _data: web::Data<AppState>,
    req: web::Json<AggregateSignatureRequest>,
) -> HttpResponse {
    // For FROST, aggregation is simple - just combine the signature shares
    // In a real implementation, this would use FROST's aggregate function
    // For now, return a mock aggregated signature
    if req.signature_shares.len() < 2 {
        return HttpResponse::BadRequest().json(AggregateSignatureResponse {
            success: false,
            signature: None,
            error: Some("Need at least 2 signature shares".to_string()),
        });
    }

    // Mock aggregation - in production, this would call FROST aggregate
    let aggregated_sig = format!("aggregated_{}", req.signature_shares.join("_"));
    
    HttpResponse::Ok().json(AggregateSignatureResponse {
        success: true,
        signature: Some(aggregated_sig),
        error: None,
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let node_id = env::var("NODE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u32>()
        .expect("NODE_ID must be a number");

    let data_dir = env::var("DATA_DIR")
        .unwrap_or_else(|_| format!("./data/node{}", node_id));

    let bind_address = env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| format!("127.0.0.1:800{}", node_id));

    let peer_nodes = env::var("PEER_NODES")
        .unwrap_or_else(|_| String::new())
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    tracing::info!("Starting MPC node {} on {}", node_id, bind_address);
    tracing::info!("Data directory: {}", data_dir);

    let tss = ThresholdSigningService::new(node_id, &data_dir, peer_nodes)
        .await
        .expect("Failed to initialize TSS service");

    let app_state = AppState {
        tss: Arc::new(tss),
    };

    tracing::info!("MPC node {} ready", node_id);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(cors)
            .wrap(Logger::default())
            .route("/health", web::get().to(health))
            .route("/api/keygen", web::post().to(keygen))
            .route("/api/aggregate-keys", web::post().to(aggregate_keys))
            .route("/api/sign-phase1", web::post().to(sign_phase1))
            .route("/api/sign-phase2", web::post().to(sign_phase2))
            .route("/api/aggregate", web::post().to(aggregate_signature))
    })
    .bind(&bind_address)?
    .run()
    .await
}