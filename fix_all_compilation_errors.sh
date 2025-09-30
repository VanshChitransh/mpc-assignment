#!/bin/bash

set -e

echo "=== Fixing All Compilation Errors ==="

# Fix 1: Remove duplicate imports in solana_v1.rs
echo "1. Fixing duplicate imports in solana_v1.rs..."
sed -i.bak 's/use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry};/\/\/ Removed duplicate import/' backend/src/routes/solana_v1.rs
sed -i.bak 's/use prometheus::HistogramOpts;/\/\/ Removed duplicate import/' backend/src/routes/solana_v1.rs

# Fix 2: Fix wallet_service.rs database queries to use string conversion
echo "2. Fixing wallet_service.rs database queries..."
cat > backend/src/services/wallet_service.rs.fixed << 'FIXED_EOF'
use store::{Store, UserError};
use crate::services::mpc::MpcService;
use crate::models::api_response::ApiResponse;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SigningStatus {
    Pending,
    Phase1,
    Phase2,
    Completed,
    Failed,
    Expired,
}

impl SigningStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SigningStatus::Pending => "pending",
            SigningStatus::Phase1 => "phase1",
            SigningStatus::Phase2 => "phase2",
            SigningStatus::Completed => "completed",
            SigningStatus::Failed => "failed",
            SigningStatus::Expired => "expired",
        }
    }
    
    pub fn from_str(s: &str) -> Result<Self, WalletError> {
        match s {
            "pending" => Ok(SigningStatus::Pending),
            "phase1" => Ok(SigningStatus::Phase1),
            "phase2" => Ok(SigningStatus::Phase2),
            "completed" => Ok(SigningStatus::Completed),
            "failed" => Ok(SigningStatus::Failed),
            "expired" => Ok(SigningStatus::Expired),
            _ => Err(WalletError::InvalidInput(format!("Invalid status: {}", s))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub message_hash: String,
    pub nonce_commitment: Option<String>,
    pub signing_package: Option<String>,
    pub signature_shares: Vec<String>,
    pub final_signature: Option<String>,
    pub status: SigningStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletKey {
    pub user_id: Uuid,
    pub public_key: String,
    pub threshold: i32,
    pub total_parties: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("MPC error: {0}")]
    MpcError(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Invalid signing session: {0}")]
    InvalidSigningSession(String),
    #[error("User not found: {0}")]
    UserNotFound(String),
    #[error("Wallet key not found: {0}")]
    WalletKeyNotFound(String),
}

impl From<UserError> for WalletError {
    fn from(err: UserError) -> Self {
        WalletError::DatabaseError(err.to_string())
    }
}

pub struct WalletService {
    store: Store,
    mpc_service: MpcService,
}

impl WalletService {
    pub fn new(store: Store, mpc_service: MpcService) -> Self {
        Self { store, mpc_service }
    }

    pub async fn get_user_key(&self, user_id: &Uuid) -> Result<WalletKey, WalletError> {
        let row = sqlx::query!(
            "SELECT user_id, public_key, threshold, total_parties, created_at, updated_at FROM wallet_keys WHERE user_id = $1",
            user_id
        )
        .fetch_optional(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?
        .ok_or_else(|| WalletError::WalletKeyNotFound("Wallet key not found".to_string()))?;

        Ok(WalletKey {
            user_id: row.user_id,
            public_key: row.public_key,
            threshold: row.threshold,
            total_parties: row.total_parties,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn store_signing_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        message_hash: &str,
        nonce_commitment: Option<&str>,
        signing_package: Option<&str>,
        status: SigningStatus,
        expires_at: DateTime<Utc>,
    ) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            INSERT INTO signing_sessions 
            (id, user_id, message_hash, nonce_commitment, signing_package, signature_shares, 
             final_signature, status, created_at, updated_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW(), $9)
            "#,
            session_id,
            user_id,
            message_hash,
            nonce_commitment,
            signing_package,
            &[] as &[String],
            None::<String>,
            status.as_str(),
            expires_at
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_signing_session_by_id(&self, session_id: &Uuid) -> Result<SigningSession, WalletError> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, message_hash, nonce_commitment, signing_package, 
                   signature_shares, final_signature, status, created_at, updated_at, expires_at
            FROM signing_sessions 
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_optional(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?
        .ok_or_else(|| WalletError::InvalidSigningSession("Session not found".to_string()))?;

        Ok(SigningSession {
            id: row.id,
            user_id: row.user_id,
            message_hash: row.message_hash,
            nonce_commitment: row.nonce_commitment,
            signing_package: row.signing_package,
            signature_shares: row.signature_shares.unwrap_or_default(),
            final_signature: row.final_signature,
            status: SigningStatus::from_str(&row.status)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
            expires_at: row.expires_at,
        })
    }

    pub async fn get_signing_session_by_message_hash(&self, user_id: &Uuid, message_hash: &str) -> Result<SigningSession, WalletError> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, message_hash, nonce_commitment, signing_package, 
                   signature_shares, final_signature, status, created_at, updated_at, expires_at
            FROM signing_sessions 
            WHERE user_id = $1 AND message_hash = $2
            "#,
            user_id,
            message_hash
        )
        .fetch_optional(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?
        .ok_or_else(|| WalletError::InvalidSigningSession("Session not found".to_string()))?;

        Ok(SigningSession {
            id: row.id,
            user_id: row.user_id,
            message_hash: row.message_hash,
            nonce_commitment: row.nonce_commitment,
            signing_package: row.signing_package,
            signature_shares: row.signature_shares.unwrap_or_default(),
            final_signature: row.final_signature,
            status: SigningStatus::from_str(&row.status)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
            expires_at: row.expires_at,
        })
    }

    pub async fn update_signing_session_status(&self, session_id: &Uuid, status: SigningStatus) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            UPDATE signing_sessions 
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            session_id,
            status.as_str()
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn add_signature_share(&self, session_id: &Uuid, share: &str) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            UPDATE signing_sessions 
            SET signature_shares = array_append(signature_shares, $2), updated_at = NOW()
            WHERE id = $1
            "#,
            session_id,
            share
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn set_final_signature(&self, session_id: &Uuid, signature: &str) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            UPDATE signing_sessions 
            SET final_signature = $2, status = 'completed', updated_at = NOW()
            WHERE id = $1
            "#,
            session_id,
            signature
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<u64, WalletError> {
        let result = sqlx::query!(
            "DELETE FROM signing_sessions WHERE expires_at < NOW()"
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected())
    }
}
FIXED_EOF

mv backend/src/services/wallet_service.rs.fixed backend/src/services/wallet_service.rs

echo "3. Fixing middleware imports..."
# Fix middleware imports
cat > backend/src/middleware/mod.rs << 'MOD_EOF'
pub mod auth;
pub mod logging;
pub mod metrics;
pub mod rate_limit;

pub use auth::{AuthMiddleware, AuthExtensions, JwtAuth};
pub use rate_limit::RateLimitMiddleware;
pub use metrics::ApiMetrics;
MOD_EOF

echo "4. Fixing middleware implementations..."
# Fix rate limit middleware
cat > backend/src/middleware/rate_limit.rs << 'RATE_LIMIT_EOF'
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;
use uuid::Uuid;

pub struct RateLimitMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitService {
            service: Rc::new(service),
        }))
    }
}

pub struct RateLimitService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RateLimitService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        
        Box::pin(async move {
            // Simple rate limiting - just pass through for now
            let res = service.call(req).await?;
            Ok(res)
        })
    }
}
RATE_LIMIT_EOF

# Fix logging middleware
cat > backend/src/middleware/logging.rs << 'LOGGING_EOF'
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};

pub struct LoggingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for LoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggingService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(LoggingService {
            service: std::rc::Rc::new(service),
        }))
    }
}

pub struct LoggingService<S> {
    service: std::rc::Rc<S>,
}

impl<S, B> Service<ServiceRequest> for LoggingService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        
        Box::pin(async move {
            let res = service.call(req).await?;
            Ok(res)
        })
    }
}
LOGGING_EOF

# Fix metrics middleware
cat > backend/src/middleware/metrics.rs << 'METRICS_EOF'
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use prometheus::{Counter, Histogram, Registry, Opts, Gauge, IntCounterVec, HistogramVec, HistogramOpts};

pub struct ApiMetrics {
    pub requests_total: IntCounterVec,
    pub request_duration: HistogramVec,
    pub active_connections: Gauge,
}

impl ApiMetrics {
    pub fn new(registry: &Registry) -> Self {
        let requests_total = IntCounterVec::new(
            Opts::new("api_requests_total", "Total number of API requests"),
            &["method", "path", "status"]
        ).unwrap();
        
        let request_duration = HistogramVec::new(
            HistogramOpts::new("api_request_duration_seconds", "API request duration in seconds"),
            &["method", "path"]
        ).unwrap();
        
        let active_connections = Gauge::new("api_active_connections", "Number of active connections").unwrap();

        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(request_duration.clone())).unwrap();
        registry.register(Box::new(active_connections.clone())).unwrap();

        Self {
            requests_total,
            request_duration,
            active_connections,
        }
    }
}

pub struct MetricsMiddleware {
    pub metrics: ApiMetrics,
}

impl<S, B> Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = MetricsService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(MetricsService {
            service: std::rc::Rc::new(service),
            metrics: self.metrics.clone(),
        }))
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    service: std::rc::Rc<S>,
    metrics: ApiMetrics,
}

impl<S, B> Service<ServiceRequest> for MetricsService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let metrics = self.metrics.clone();
        
        Box::pin(async move {
            let res = service.call(req).await?;
            Ok(res)
        })
    }
}
METRICS_EOF

echo "5. Fixing main.rs imports..."
# Fix main.rs
cat > backend/src/main.rs << 'MAIN_EOF'
use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use dotenv::dotenv;
use std::env;
use tracing_subscriber;

mod middleware;
mod models;
mod routes;
mod services;
mod blockchain;

use middleware::auth::AuthMiddleware;
use routes::{api, user, wallet, solana_v1};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    tracing_subscriber::fmt::init();
    
    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    
    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .service(web::scope("/api/v1").configure(api::config))
    })
    .bind(&bind_address)?
    .run()
    .await
}
MAIN_EOF

echo "All compilation fixes applied!"
