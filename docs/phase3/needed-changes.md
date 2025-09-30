// ============================================================================
// FIX 1: Response Borrow Checker Errors (E0382)
// Location: backend/src/services/mpc.rs
// Issue: Using response after calling response.text()
// ============================================================================

// BEFORE (BROKEN):
/*
let status = response.status();
if !status.is_success() {
    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
    return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
}
*/

// AFTER (FIXED):
// Save status BEFORE consuming response
async fn send_keygen_request(
    client: &Client,
    node_url: &str,
    request: &KeyGenRequest,
    timeout: Duration,
) -> Result<KeyGenResponse, MpcError> {
    let url = format!("{}/api/keygen", node_url);
    
    let response = client
        .post(&url)
        .timeout(timeout)
        .json(request)
        .send()
        .await?;

    let status = response.status(); // Save status first
    
    if !status.is_success() {
        // Now consume response for error text
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
    }

    // Only call json() if status is success
    Ok(response.json().await?)
}

// Apply the same fix to all HTTP request functions:

async fn send_aggregate_request(
    client: &Client,
    node_url: &str,
    request: &AggregateKeysRequest,
    timeout: Duration,
) -> Result<AggregateKeysResponse, MpcError> {
    let url = format!("{}/api/aggregate-keys", node_url);
    
    let response = client
        .post(&url)
        .timeout(timeout)
        .json(request)
        .send()
        .await?;

    let status = response.status(); // Save status first
    
    if !status.is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
    }

    Ok(response.json().await?)
}

async fn send_sign_phase1_request(
    client: &Client,
    node_url: &str,
    request: &SignPhase1Request,
    timeout: Duration,
) -> Result<SignPhase1Response, MpcError> {
    let url = format!("{}/api/sign-phase1", node_url);
    
    let response = client
        .post(&url)
        .timeout(timeout)
        .json(request)
        .send()
        .await?;

    let status = response.status(); // Save status first
    
    if !status.is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
    }

    Ok(response.json().await?)
}

async fn send_sign_phase2_request(
    client: &Client,
    node_url: &str,
    request: &SignPhase2Request,
    timeout: Duration,
) -> Result<SignPhase2Response, MpcError> {
    let url = format!("{}/api/sign-phase2", node_url);
    
    let response = client
        .post(&url)
        .timeout(timeout)
        .json(request)
        .send()
        .await?;

    let status = response.status(); // Save status first
    
    if !status.is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
    }

    Ok(response.json().await?)
}

async fn send_aggregate_signature_request(
    client: &Client,
    node_url: &str,
    request: &AggregateSignatureRequest,
    timeout: Duration,
) -> Result<AggregateSignatureResponse, MpcError> {
    let url = format!("{}/api/aggregate", node_url);
    
    let response = client
        .post(&url)
        .timeout(timeout)
        .json(request)
        .send()
        .await?;

    let status = response.status(); // Save status first
    
    if !status.is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
    }

    Ok(response.json().await?)
}

// ============================================================================
// FIX 2: Add Serialize to MpcError (E0277)
// Location: backend/src/services/mpc.rs
// Issue: MpcError doesn't implement Serialize
// ============================================================================

// BEFORE (BROKEN):
/*
#[derive(Error, Debug)]
pub enum MpcError {
    ...
}
*/

// AFTER (FIXED):
use serde::{Deserialize, Serialize};

#[derive(Error, Debug, Serialize, Deserialize, Clone)] // Added Serialize, Deserialize, Clone
pub enum MpcError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String), // Changed from reqwest::Error to String (reqwest::Error not serializable)
    #[error("MPC node error: {0}")]
    NodeError(String),
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Insufficient nodes responded: {available}/{required}")]
    InsufficientNodes { available: usize, required: usize },
    #[error("Timeout waiting for MPC response")]
    Timeout,
    #[error("All nodes are unavailable")]
    AllNodesDown,
    #[error("Invalid signature format")]
    InvalidSignatureFormat,
    #[error("Aggregation failed: {0}")]
    AggregationFailed(String),
    #[error("Circuit breaker open for node: {0}")]
    CircuitBreakerOpen(String),
    #[error("Max retries exceeded: {0}")]
    MaxRetriesExceeded(String),
}

// Update From implementation to convert reqwest::Error to String
impl From<reqwest::Error> for MpcError {
    fn from(err: reqwest::Error) -> Self {
        MpcError::RequestFailed(err.to_string())
    }
}

// ============================================================================
// FIX 3: Fix wallet_service.rs ClusterStatus field access (E0609)
// Location: backend/src/services/wallet_service.rs:208
// Issue: Accessing non-existent field cluster_status.is_operational
// ============================================================================

// BEFORE (BROKEN):
/*
"cluster_operational": cluster_status.is_operational,
*/

// AFTER (FIXED):
// Use threshold_met instead of is_operational
"cluster_operational": cluster_status.threshold_met,

// Full context of the fix in wallet_service.rs:
pub async fn check_health(&self, user_id: Uuid) -> Result<HealthCheckResponse, WalletError> {
    // Check if user has keys
    let has_keys = match self.store.get_user_by_id(&user_id).await {
        Ok(user) => user.public_key.is_some(),
        Err(_) => false,
    };

    // Get MPC cluster status
    let cluster_status = self.mpc_client.get_cluster_status().await
        .map_err(|e| WalletError::MpcError(e.to_string()))?;

    Ok(HealthCheckResponse {
        success: true,
        status: if cluster_status.threshold_met && has_keys {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        user_has_keys: has_keys,
        cluster_status: Some(serde_json::json!({
            "status": cluster_status.status,
            "healthy_nodes": cluster_status.healthy_nodes,
            "total_nodes": cluster_status.total_nodes,
            "threshold": cluster_status.threshold,
            "cluster_operational": cluster_status.threshold_met, // FIXED: Use threshold_met
        })),
    })
}

// ============================================================================
// FIX 4: Fix rate_limit.rs type mismatch (E0308)
// Location: backend/src/middleware/rate_limit.rs:122
// Issue: Type mismatch between HttpResponse<B> and HttpResponse<BoxBody>
// ============================================================================

// This is an actix-web version compatibility issue
// The fix depends on your actix-web version

// For actix-web 4.x:
use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};

// Update the call method signature:
impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>; // Keep generic B, not BoxBody
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let limiter = self.limiter.clone();

        Box::pin(async move {
            // Get client identifier
            let client_id = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string();

            // Check rate limit
            if !limiter.check_rate_limit(&client_id).await {
                return Ok(req.into_response(
                    HttpResponse::TooManyRequests()
                        .json(serde_json::json!({
                            "error": "Rate limit exceeded"
                        }))
                        .map_into_boxed_body() // Convert to BoxBody here if needed
                ));
            }

            // Continue with request
            service.call(req).await
        })
    }
}

// ============================================================================
// FIX 5: Fix test imports (E0433)
// Location: backend/tests/test_step_3_1_complete.rs:4
// Issue: Can't resolve backend crate in tests
// ============================================================================

// BEFORE (BROKEN):
/*
use backend::services::mpc::{...};
*/

// AFTER (FIXED):
// Tests need to import from the library root, not the binary

// Update Cargo.toml to ensure lib is available:
// backend/Cargo.toml should have:
/*
[lib]
name = "backend"
path = "src/lib.rs"

[[bin]]
name = "backend"
path = "src/main.rs"
*/

// Then in tests, import like this:
use backend::services::mpc::{
    MpcClient, MpcError, RetryConfig, LoadBalancingStrategy, ClusterStatus,
};

// If backend/src/lib.rs doesn't exist, create it:
// backend/src/lib.rs:
pub mod services;
pub mod routes;
pub mod middleware;
pub mod models;
pub mod error;

// Make sure services/mod.rs exports mpc module:
// backend/src/services/mod.rs:
pub mod mpc;
pub mod jupiter;
pub mod solana;
// ... other modules

// ============================================================================
// COMPLETE FIX SUMMARY
// ============================================================================

/*
1. ✅ Fixed borrow checker errors in all HTTP request functions
   - Save status before consuming response
   - Applied to: send_keygen_request, send_aggregate_request, 
     send_sign_phase1_request, send_sign_phase2_request, 
     send_aggregate_signature_request

2. ✅ Added Serialize/Deserialize to MpcError
   - Changed RequestFailed to use String instead of reqwest::Error
   - Added proper From implementation

3. ✅ Fixed wallet_service.rs cluster status access
   - Changed cluster_status.is_operational to cluster_status.threshold_met

4. ✅ Fixed rate_limit.rs type mismatch
   - Kept generic type parameter B
   - Added map_into_boxed_body() where needed

5. ✅ Fixed test imports
   - Ensured [lib] section in Cargo.toml
   - Created lib.rs with proper module exports
   - Updated test imports
*/