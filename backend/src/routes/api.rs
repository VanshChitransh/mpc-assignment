use crate::AppState;
use crate::services::{WalletService, WalletError};
use crate::services::wallet_service::{KeyGenRequest, SignPhase1Request, SignPhase2Request, AggregateRequest};
use crate::models::api_response::{ApiResponse, error_codes};
use actix_web::{web, HttpRequest, HttpResponse, Result, HttpMessage};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;
use utoipa::{OpenApi, ToSchema};

/// OpenAPI documentation for wallet API
#[derive(OpenApi)]
#[openapi(
    paths(
        keygen,
        sign_phase1,
        sign_phase2,
        aggregate,
        health
    ),
    components(
        schemas(
            KeyGenRequestSchema,
            SignPhase1RequestSchema,
            SignPhase2RequestSchema,
            AggregateRequestSchema,
            KeyGenResponse,
            SignPhase1Response,
            SignPhase2Response,
            AggregateResponse,
            HealthResponse,
            ApiResponseSchema
        )
    ),
    tags(
        (name = "wallet", description = "Wallet operations API")
    )
)]
pub struct ApiDoc;

// Request schemas for OpenAPI
#[derive(Serialize, Deserialize, ToSchema)]
pub struct KeyGenRequestSchema {
    pub threshold: u32,
    pub participants: u32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SignPhase1RequestSchema {
    pub message: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SignPhase2RequestSchema {
    pub session_id: String,
    pub nonce_commitment: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AggregateRequestSchema {
    pub session_id: String,
    pub signature_shares: Vec<String>,
}

// Response structures for wallet operations
#[derive(Serialize, ToSchema)]
pub struct KeyGenResponse {
    pub public_key: String,
}

#[derive(Serialize, ToSchema)]
pub struct SignPhase1Response {
    pub session_id: String,
    pub nonce_commitment: String,
    pub signing_package: String,
}

#[derive(Serialize, ToSchema)]
pub struct SignPhase2Response {
    pub signature_share: String,
}

#[derive(Serialize, ToSchema)]
pub struct AggregateResponse {
    pub signature: String,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub cluster_status: Option<serde_json::Value>,
}

#[derive(Serialize, ToSchema)]
pub struct ApiResponseSchema {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}

/// POST /api/v1/wallet/keygen - Generate distributed keys for a user
#[utoipa::path(
    post,
    path = "/api/v1/wallet/keygen",
    request_body = KeyGenRequestSchema,
    responses(
        (status = 200, description = "Key generation successful", body = ApiResponseSchema),
        (status = 400, description = "Bad request", body = ApiResponseSchema),
        (status = 401, description = "Unauthorized", body = ApiResponseSchema),
        (status = 409, description = "Key already exists", body = ApiResponseSchema),
        (status = 500, description = "Internal server error", body = ApiResponseSchema)
    ),
    tag = "wallet"
)]
pub async fn keygen(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<KeyGenRequest>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let keygen_req = req_body.into_inner();
    
    info!("Processing API keygen request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.generate_key(user_id, keygen_req).await {
        Ok(response) => {
            info!("Successfully processed API keygen request for user {}", user_id);
            let api_response = ApiResponse::success(KeyGenResponse {
                public_key: response.public_key.unwrap_or_default(),
            });
            Ok(api_response.to_http_response(actix_web::http::StatusCode::OK))
        }
        Err(e) => {
            error!("API key generation failed for user {}: {}", user_id, e);
            let api_response: ApiResponse<KeyGenResponse> = ApiResponse::error(error_codes::WALLET_ERROR, &e.to_string());
            Ok(api_response.to_http_response(e.status_code()))
        }
    }
}

/// POST /api/v1/wallet/sign/phase1 - Generate nonce commitments for signing
#[utoipa::path(
    post,
    path = "/api/v1/wallet/sign/phase1",
    request_body = SignPhase1RequestSchema,
    responses(
        (status = 200, description = "Sign phase 1 successful", body = ApiResponseSchema),
        (status = 400, description = "Bad request", body = ApiResponseSchema),
        (status = 401, description = "Unauthorized", body = ApiResponseSchema),
        (status = 500, description = "Internal server error", body = ApiResponseSchema)
    ),
    tag = "wallet"
)]
pub async fn sign_phase1(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<SignPhase1Request>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let sign_req = req_body.into_inner();
    
    info!("Processing API sign phase1 request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.sign_phase1(user_id, sign_req).await {
        Ok(response) => {
            info!("Successfully processed API sign phase1 request for user {}", user_id);
            let api_response = ApiResponse::success(SignPhase1Response {
                session_id: response.session_id.unwrap_or_default(),
                nonce_commitment: response.nonce_commitment.unwrap_or_default(),
                signing_package: response.signing_package.unwrap_or_default(),
            });
            Ok(api_response.to_http_response(actix_web::http::StatusCode::OK))
        }
        Err(e) => {
            error!("API sign phase1 failed for user {}: {}", user_id, e);
            let api_response: ApiResponse<SignPhase1Response> = ApiResponse::error(error_codes::WALLET_ERROR, &e.to_string());
            Ok(api_response.to_http_response(e.status_code()))
        }
    }
}

/// POST /api/v1/wallet/sign/phase2 - Generate signature shares
#[utoipa::path(
    post,
    path = "/api/v1/wallet/sign/phase2",
    request_body = SignPhase2RequestSchema,
    responses(
        (status = 200, description = "Sign phase 2 successful", body = ApiResponseSchema),
        (status = 400, description = "Bad request", body = ApiResponseSchema),
        (status = 401, description = "Unauthorized", body = ApiResponseSchema),
        (status = 500, description = "Internal server error", body = ApiResponseSchema)
    ),
    tag = "wallet"
)]
pub async fn sign_phase2(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<SignPhase2Request>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let sign_req = req_body.into_inner();
    
    info!("Processing API sign phase2 request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.sign_phase2(user_id, sign_req).await {
        Ok(response) => {
            info!("Successfully processed API sign phase2 request for user {}", user_id);
            let api_response = ApiResponse::success(SignPhase2Response {
                signature_share: response.signature_share.unwrap_or_default(),
            });
            Ok(api_response.to_http_response(actix_web::http::StatusCode::OK))
        }
        Err(e) => {
            error!("API sign phase2 failed for user {}: {}", user_id, e);
            let api_response: ApiResponse<SignPhase2Response> = ApiResponse::error(error_codes::WALLET_ERROR, &e.to_string());
            Ok(api_response.to_http_response(e.status_code()))
        }
    }
}

/// POST /api/v1/wallet/aggregate - Aggregate signature shares into final signature
#[utoipa::path(
    post,
    path = "/api/v1/wallet/aggregate",
    request_body = AggregateRequestSchema,
    responses(
        (status = 200, description = "Aggregation successful", body = ApiResponseSchema),
        (status = 400, description = "Bad request", body = ApiResponseSchema),
        (status = 401, description = "Unauthorized", body = ApiResponseSchema),
        (status = 500, description = "Internal server error", body = ApiResponseSchema)
    ),
    tag = "wallet"
)]
pub async fn aggregate(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<AggregateRequest>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let agg_req = req_body.into_inner();
    
    info!("Processing API aggregate request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.aggregate_signature(user_id, agg_req).await {
        Ok(response) => {
            info!("Successfully processed API aggregate request for user {}", user_id);
            let api_response = ApiResponse::success(AggregateResponse {
                signature: response.signature.unwrap_or_default(),
            });
            Ok(api_response.to_http_response(actix_web::http::StatusCode::OK))
        }
        Err(e) => {
            error!("API aggregate failed for user {}: {}", user_id, e);
            let api_response: ApiResponse<AggregateResponse> = ApiResponse::error(error_codes::WALLET_ERROR, &e.to_string());
            Ok(api_response.to_http_response(e.status_code()))
        }
    }
}

/// GET /api/v1/wallet/health - Check MPC cluster health
#[utoipa::path(
    get,
    path = "/api/v1/wallet/health",
    responses(
        (status = 200, description = "Health check successful", body = ApiResponseSchema),
        (status = 401, description = "Unauthorized", body = ApiResponseSchema),
        (status = 500, description = "Internal server error", body = ApiResponseSchema)
    ),
    tag = "wallet"
)]
pub async fn health(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    
    info!("Processing API health check request for user {}", user_id);
    
    // Create wallet service instance
    let wallet_service = WalletService::new(data.mpc_client.clone(), data.store.clone());
    
    // Delegate to wallet service
    match wallet_service.check_health(user_id).await {
        Ok(response) => {
            info!("Successfully processed API health check request for user {}", user_id);
            let api_response = ApiResponse::success(HealthResponse {
                status: response.status,
                cluster_status: response.cluster_status,
            });
            Ok(api_response.to_http_response(actix_web::http::StatusCode::OK))
        }
        Err(e) => {
            error!("API health check failed for user {}: {}", user_id, e);
            let api_response: ApiResponse<HealthResponse> = ApiResponse::error(error_codes::WALLET_ERROR, &e.to_string());
            Ok(api_response.to_http_response(e.status_code()))
        }
    }
}

/// Register all API routes
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/wallet")
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
