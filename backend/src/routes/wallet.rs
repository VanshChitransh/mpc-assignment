use crate::AppState;
use crate::services::{WalletService, WalletError};
use crate::services::wallet_service::{KeyGenRequest, SignPhase1Request, SignPhase2Request, AggregateRequest};
use actix_web::{web, HttpRequest, HttpResponse, Result, HttpMessage};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

// Response structures for wallet operations
#[derive(Serialize)]
pub struct KeyGenResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SignPhase1Response {
    pub success: bool,
    pub session_id: Option<String>,
    pub nonce_commitment: Option<String>,
    pub signing_package: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SignPhase2Response {
    pub success: bool,
    pub signature_share: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct AggregateResponse {
    pub success: bool,
    pub signature: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub success: bool,
    pub status: String,
    pub cluster_status: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

/// POST /wallet/keygen - Generate distributed keys for a user
pub async fn keygen(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<KeyGenRequest>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let keygen_req = req_body.into_inner();
    
    info!("Processing keygen request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.generate_key(user_id, keygen_req).await {
        Ok(response) => {
            info!("Successfully processed keygen request for user {}", user_id);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Key generation failed for user {}: {}", user_id, e);
            Ok(HttpResponse::build(e.status_code()).json(KeyGenResponse {
                success: false,
                public_key: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// POST /wallet/sign/phase1 - Generate nonce commitments for signing
pub async fn sign_phase1(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<SignPhase1Request>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let sign_req = req_body.into_inner();
    
    info!("Processing sign phase1 request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.sign_phase1(user_id, sign_req).await {
        Ok(response) => {
            info!("Successfully processed sign phase1 request for user {}", user_id);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Sign phase1 failed for user {}: {}", user_id, e);
            Ok(HttpResponse::build(e.status_code()).json(SignPhase1Response {
                success: false,
                session_id: None,
                nonce_commitment: None,
                signing_package: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// POST /wallet/sign/phase2 - Generate signature shares
pub async fn sign_phase2(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<SignPhase2Request>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let sign_req = req_body.into_inner();
    
    info!("Processing sign phase2 request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.sign_phase2(user_id, sign_req).await {
        Ok(response) => {
            info!("Successfully processed sign phase2 request for user {}", user_id);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Sign phase2 failed for user {}: {}", user_id, e);
            Ok(HttpResponse::build(e.status_code()).json(SignPhase2Response {
                success: false,
                signature_share: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// POST /wallet/aggregate - Aggregate signature shares into final signature
pub async fn aggregate(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<AggregateRequest>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let agg_req = req_body.into_inner();
    
    info!("Processing aggregate request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.aggregate_signature(user_id, agg_req).await {
        Ok(response) => {
            info!("Successfully processed aggregate request for user {}", user_id);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Aggregate failed for user {}: {}", user_id, e);
            Ok(HttpResponse::build(e.status_code()).json(AggregateResponse {
                success: false,
                signature: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// GET /wallet/health - Check MPC cluster health
pub async fn health(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    
    info!("Processing health check request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.check_health(user_id).await {
        Ok(response) => {
            info!("Successfully processed health check request for user {}", user_id);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Health check failed for user {}: {}", user_id, e);
            Ok(HttpResponse::build(e.status_code()).json(HealthResponse {
                success: false,
                status: "error".to_string(),
                cluster_status: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Register all wallet routes
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/wallet")
            .route("/keygen", web::post().to(keygen))
            .route("/sign/phase1", web::post().to(sign_phase1))
            .route("/sign/phase2", web::post().to(sign_phase2))
            .route("/aggregate", web::post().to(aggregate))
            .route("/health", web::get().to(health))
    );
}

// Helper trait to get status code from WalletError
trait WalletErrorExt {
    fn status_code(&self) -> actix_web::http::StatusCode;
}

impl WalletErrorExt for WalletError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        
        match self {
            WalletError::UserNotFound(_) => StatusCode::NOT_FOUND,
            WalletError::KeyAlreadyExists(_) => StatusCode::CONFLICT,
            WalletError::NoKeysFound(_) => StatusCode::NOT_FOUND,
            WalletError::InvalidSigningSession(_) => StatusCode::BAD_REQUEST,
            WalletError::SigningSessionExpired => StatusCode::GONE,
            WalletError::InsufficientSignatureShares { .. } => StatusCode::BAD_REQUEST,
            WalletError::InvalidSignatureFormat => StatusCode::BAD_REQUEST,
            WalletError::ReplayAttack => StatusCode::CONFLICT,
            WalletError::ClusterUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            WalletError::RetryLimitExceeded => StatusCode::SERVICE_UNAVAILABLE,
            WalletError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            WalletError::MpcError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            WalletError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
