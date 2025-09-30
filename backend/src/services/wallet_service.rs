use crate::services::mpc::{MpcClient, MpcError};
use store::{Store, UserError};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use thiserror::Error;

/// Wallet-specific error types
#[derive(Error, Debug)]
pub enum WalletError {
    #[error("MPC operation failed: {0}")]
    MpcError(#[from] MpcError),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("User not found: {0}")]
    UserNotFound(Uuid),
    #[error("Key generation already exists for user: {0}")]
    KeyAlreadyExists(Uuid),
    #[error("No keys found for user: {0}")]
    NoKeysFound(Uuid),
    #[error("Invalid signing session: {0}")]
    InvalidSigningSession(String),
    #[error("Signing session expired")]
    SigningSessionExpired,
    #[error("Insufficient signature shares: {available}/{required}")]
    InsufficientSignatureShares { available: usize, required: usize },
    #[error("Invalid signature format")]
    InvalidSignatureFormat,
    #[error("Replay attack detected")]
    ReplayAttack,
    #[error("MPC cluster unavailable")]
    ClusterUnavailable,
    #[error("Retry limit exceeded")]
    RetryLimitExceeded,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Wallet state management structures
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WalletKey {
    pub user_id: Uuid,
    pub public_key: String,
    pub threshold: u32,
    pub total_parties: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "signing_status", rename_all = "lowercase")]
pub enum SigningStatus {
    Phase1,
    Phase2,
    Completed,
    Failed,
    Expired,
}

/// Request/Response structures for wallet operations
#[derive(Debug, Deserialize)]
pub struct KeyGenRequest {
    pub threshold: Option<u32>,
    pub total_parties: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct KeyGenResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SignPhase1Request {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SignPhase1Response {
    pub success: bool,
    pub session_id: Option<String>,
    pub nonce_commitment: Option<String>,
    pub signing_package: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SignPhase2Request {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SignPhase2Response {
    pub success: bool,
    pub signature_share: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AggregateRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct AggregateResponse {
    pub success: bool,
    pub signature: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub success: bool,
    pub status: String,
    pub cluster_status: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Configuration for retry logic
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Main wallet service that orchestrates MPC operations
pub struct WalletService {
    mpc_client: MpcClient,
    store: Store,
    retry_config: RetryConfig,
}

impl WalletService {
    pub fn new(mpc_client: MpcClient, store: Store) -> Self {
        Self {
            mpc_client,
            store,
            retry_config: RetryConfig::default(),
        }
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Generate distributed keys for a user with idempotency
    pub async fn generate_key(
        &self,
        user_id: Uuid,
        request: KeyGenRequest,
    ) -> Result<KeyGenResponse, WalletError> {
        info!("Processing key generation request for user {}", user_id);

        // Check if user already has keys
        if let Ok(existing_key) = self.get_user_key(&user_id).await {
            warn!("User {} already has keys, returning existing key", user_id);
            return Ok(KeyGenResponse {
                success: true,
                public_key: Some(existing_key.public_key),
                error: None,
            });
        }

        // Validate user exists
        self.validate_user_exists(&user_id).await?;

        // Use default values if not provided
        let threshold = request.threshold.unwrap_or(2);
        let total_parties = request.total_parties.unwrap_or(3);

        if threshold > total_parties {
            return Err(WalletError::InvalidInput(
                "Threshold cannot be greater than total parties".to_string(),
            ));
        }

        // Generate key with retry logic
        let public_key = self
            .retry_mpc_operation(|| async {
                self.mpc_client.generate_key(&user_id).await
            })
            .await?;

        // Store the key in database
        self.store_wallet_key(user_id, &public_key, threshold, total_parties)
            .await?;

        info!("Successfully generated key for user {}", user_id);
        Ok(KeyGenResponse {
            success: true,
            public_key: Some(public_key),
            error: None,
        })
    }

    /// Phase 1: Generate nonce commitments for signing
    pub async fn sign_phase1(
        &self,
        user_id: Uuid,
        request: SignPhase1Request,
    ) -> Result<SignPhase1Response, WalletError> {
        info!("Processing sign phase1 request for user {}", user_id);

        // Validate user has keys
        let wallet_key = self.get_user_key(&user_id).await?;

        // Validate input
        if request.message.is_empty() {
            return Err(WalletError::InvalidInput("Message cannot be empty".to_string()));
        }

        // Create message hash for idempotency
        let message_hash = self.create_message_hash(&request.message);

        // Check for existing session (idempotency)
        if let Ok(existing_session) = self.get_signing_session(&user_id, &message_hash).await {
            if existing_session.status == SigningStatus::Phase1 || existing_session.status == SigningStatus::Phase2 {
                info!("Returning existing signing session for user {}", user_id);
                return Ok(SignPhase1Response {
                    success: true,
                    session_id: Some(existing_session.id.to_string()),
                    nonce_commitment: existing_session.nonce_commitment,
                    signing_package: existing_session.signing_package,
                    error: None,
                });
            }
        }

        // Check MPC cluster availability
        if !self.mpc_client.check_threshold_availability().await {
            return Err(WalletError::ClusterUnavailable);
        }

        // Create new signing session
        let session_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::minutes(30);

        // For now, simulate phase1 response since MPC client doesn't expose individual phases
        // In a real implementation, you'd call the MPC client's phase1 method
        let nonce_commitment = "mock_nonce_commitment".to_string();
        let signing_package = "mock_signing_package".to_string();

        // Store session in database
        self.store_signing_session(
            session_id,
            user_id,
            &message_hash,
            Some(&nonce_commitment),
            Some(&signing_package),
            SigningStatus::Phase1,
            expires_at,
        )
        .await?;

        info!("Phase1 signing initiated for user {}", user_id);
        Ok(SignPhase1Response {
            success: true,
            session_id: Some(session_id.to_string()),
            nonce_commitment: Some(nonce_commitment),
            signing_package: Some(signing_package),
            error: None,
        })
    }

    /// Phase 2: Generate signature shares
    pub async fn sign_phase2(
        &self,
        user_id: Uuid,
        request: SignPhase2Request,
    ) -> Result<SignPhase2Response, WalletError> {
        info!("Processing sign phase2 request for user {}", user_id);

        // Parse session ID
        let session_id = Uuid::parse_str(&request.session_id)
            .map_err(|_| WalletError::InvalidInput("Invalid session ID".to_string()))?;

        // Get signing session
        let mut session = self.get_signing_session_by_id(&session_id).await?;

        // Validate session belongs to user
        if session.user_id != user_id {
            return Err(WalletError::InvalidSigningSession("Session does not belong to user".to_string()));
        }

        // Validate session status
        if session.status != SigningStatus::Phase1 {
            return Err(WalletError::InvalidSigningSession(
                format!("Invalid session status: {:?}", session.status)
            ));
        }

        // Check if session expired
        if Utc::now() > session.expires_at {
            self.update_session_status(&session_id, SigningStatus::Expired).await?;
            return Err(WalletError::SigningSessionExpired);
        }

        // Validate input
        if request.message.is_empty() {
            return Err(WalletError::InvalidInput("Message cannot be empty".to_string()));
        }

        // Check MPC cluster availability
        if !self.mpc_client.check_threshold_availability().await {
            return Err(WalletError::ClusterUnavailable);
        }

        // Generate signature share with retry logic
        let signature_share = self
            .retry_mpc_operation(|| async {
                // For now, simulate signature share generation
                // In a real implementation, you'd call the MPC client's phase2 method
                Ok("mock_signature_share".to_string())
            })
            .await?;

        // Update session with signature share
        self.add_signature_share(&session_id, &signature_share).await?;
        self.update_session_status(&session_id, SigningStatus::Phase2).await?;

        info!("Phase2 signing completed for user {}", user_id);
        Ok(SignPhase2Response {
            success: true,
            signature_share: Some(signature_share),
            error: None,
        })
    }

    /// Aggregate signature shares into final signature
    pub async fn aggregate_signature(
        &self,
        user_id: Uuid,
        request: AggregateRequest,
    ) -> Result<AggregateResponse, WalletError> {
        info!("Processing aggregate request for user {}", user_id);

        // Parse session ID
        let session_id = Uuid::parse_str(&request.session_id)
            .map_err(|_| WalletError::InvalidInput("Invalid session ID".to_string()))?;

        // Get signing session
        let session = self.get_signing_session_by_id(&session_id).await?;

        // Validate session belongs to user
        if session.user_id != user_id {
            return Err(WalletError::InvalidSigningSession("Session does not belong to user".to_string()));
        }

        // Validate session status
        if session.status != SigningStatus::Phase2 {
            return Err(WalletError::InvalidSigningSession(
                format!("Invalid session status: {:?}", session.status)
            ));
        }

        // Check if session expired
        if Utc::now() > session.expires_at {
            self.update_session_status(&session_id, SigningStatus::Expired).await?;
            return Err(WalletError::SigningSessionExpired);
        }

        // Validate we have enough signature shares
        if session.signature_shares.len() < 2 {
            return Err(WalletError::InsufficientSignatureShares {
                available: session.signature_shares.len(),
                required: 2,
            });
        }

        // Check MPC cluster availability
        if !self.mpc_client.check_threshold_availability().await {
            return Err(WalletError::ClusterUnavailable);
        }

        // Aggregate signature with retry logic
        let final_signature = self
            .retry_mpc_operation(|| async {
                // For now, simulate signature aggregation
                // In a real implementation, you'd call the MPC client's aggregate method
                Ok("mock_final_signature".to_string())
            })
            .await?;

        // Update session with final signature
        self.update_session_final_signature(&session_id, &final_signature).await?;
        self.update_session_status(&session_id, SigningStatus::Completed).await?;

        info!("Signature aggregation completed for user {}", user_id);
        Ok(AggregateResponse {
            success: true,
            signature: Some(final_signature),
            error: None,
        })
    }

    /// Check MPC cluster health
    pub async fn check_health(&self, user_id: Uuid) -> Result<HealthResponse, WalletError> {
        info!("Processing health check request for user {}", user_id);

        // Validate user exists
        self.validate_user_exists(&user_id).await?;

        // Get cluster status
        let cluster_status = self.mpc_client.get_cluster_status().await;
        let is_healthy = cluster_status.is_operational;
        let status = if is_healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        };

        info!("MPC cluster health check completed: {}", status);
        Ok(HealthResponse {
            success: true,
            status,
            cluster_status: Some(serde_json::to_value(cluster_status).unwrap_or_default()),
            error: None,
        })
    }

    // Private helper methods

    async fn validate_user_exists(&self, user_id: &Uuid) -> Result<(), WalletError> {
        sqlx::query_scalar!("SELECT id FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.store.pool)
            .await
            .map_err(|e| WalletError::DatabaseError(e.to_string()))?
            .ok_or_else(|| WalletError::UserNotFound(*user_id))?;
        Ok(())
    }

    async fn get_user_key(&self, user_id: &Uuid) -> Result<WalletKey, WalletError> {
        sqlx::query_as!(
            WalletKey,
            "SELECT user_id, public_key, threshold, total_parties, created_at, updated_at FROM wallet_keys WHERE user_id = $1",
            user_id
        )
        .fetch_optional(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?
        .ok_or_else(|| WalletError::NoKeysFound(*user_id))
    }

    async fn store_wallet_key(
        &self,
        user_id: Uuid,
        public_key: &str,
        threshold: u32,
        total_parties: u32,
    ) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            INSERT INTO wallet_keys (user_id, public_key, threshold, total_parties, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                public_key = EXCLUDED.public_key,
                threshold = EXCLUDED.threshold,
                total_parties = EXCLUDED.total_parties,
                updated_at = NOW()
            "#,
            user_id,
            public_key,
            threshold as i32,
            total_parties as i32
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_signing_session(
        &self,
        user_id: &Uuid,
        message_hash: &str,
    ) -> Result<SigningSession, WalletError> {
        sqlx::query_as!(
            SigningSession,
            r#"
            SELECT id, user_id, message_hash, nonce_commitment, signing_package, 
                   signature_shares, final_signature, status, created_at, updated_at, expires_at
            FROM signing_sessions 
            WHERE user_id = $1 AND message_hash = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            user_id,
            message_hash
        )
        .fetch_optional(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?
        .ok_or_else(|| WalletError::InvalidSigningSession("Session not found".to_string()))
    }

    async fn get_signing_session_by_id(&self, session_id: &Uuid) -> Result<SigningSession, WalletError> {
        sqlx::query_as!(
            SigningSession,
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
        .ok_or_else(|| WalletError::InvalidSigningSession("Session not found".to_string()))
    }

    async fn store_signing_session(
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
            &[] as &[String], // Empty signature shares array
            None::<String>,
            status as SigningStatus,
            expires_at
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn add_signature_share(&self, session_id: &Uuid, signature_share: &str) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            UPDATE signing_sessions 
            SET signature_shares = array_append(signature_shares, $2),
                updated_at = NOW()
            WHERE id = $1
            "#,
            session_id,
            signature_share
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn update_session_status(&self, session_id: &Uuid, status: SigningStatus) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            UPDATE signing_sessions 
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            session_id,
            status as SigningStatus
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn update_session_final_signature(&self, session_id: &Uuid, final_signature: &str) -> Result<(), WalletError> {
        sqlx::query!(
            r#"
            UPDATE signing_sessions 
            SET final_signature = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            session_id,
            final_signature
        )
        .execute(&self.store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn create_message_hash(&self, message: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(message.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    async fn retry_mpc_operation<F, Fut, T>(&self, operation: F) -> Result<T, WalletError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, MpcError>>,
    {
        let mut delay = Duration::from_millis(self.retry_config.base_delay_ms);
        
        for attempt in 0..=self.retry_config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt == self.retry_config.max_retries {
                        error!("MPC operation failed after {} retries: {}", self.retry_config.max_retries, e);
                        return Err(WalletError::RetryLimitExceeded);
                    }
                    
                    warn!("MPC operation attempt {} failed: {}, retrying in {:?}", attempt + 1, e, delay);
                    sleep(delay).await;
                    
                    delay = Duration::from_millis(
                        ((delay.as_millis() as f64 * self.retry_config.backoff_multiplier) as u64)
                            .min(self.retry_config.max_delay_ms)
                    );
                }
            }
        }
        
        Err(WalletError::RetryLimitExceeded)
    }
}

/// Convert WalletError to Actix ResponseError
impl actix_web::ResponseError for WalletError {
    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::http::StatusCode;
        
        let (status, error_message) = match self {
            WalletError::UserNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            WalletError::KeyAlreadyExists(_) => (StatusCode::CONFLICT, self.to_string()),
            WalletError::NoKeysFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            WalletError::InvalidSigningSession(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::SigningSessionExpired => (StatusCode::GONE, self.to_string()),
            WalletError::InsufficientSignatureShares { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::InvalidSignatureFormat => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::ReplayAttack => (StatusCode::CONFLICT, self.to_string()),
            WalletError::ClusterUnavailable => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            WalletError::RetryLimitExceeded => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            WalletError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::MpcError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            WalletError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        actix_web::HttpResponse::build(status).json(serde_json::json!({
            "success": false,
            "error": error_message
        }))
    }
}
