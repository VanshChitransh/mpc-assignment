use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info, warn};
use uuid::Uuid;
use std::time::Duration;
use std::collections::HashMap;
use futures::future::join_all;

#[derive(Error, Debug)]
pub enum MpcError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
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
}

// Request/Response structures for MPC operations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyGenRequest {
    pub user_id: String,
    pub threshold: u32,
    pub total_parties: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyGenResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggregateKeysRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggregateKeysResponse {
    pub success: bool,
    pub public_key: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase1Request {
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase1Response {
    pub success: bool,
    pub nonce_commitment: Option<String>,
    pub signing_package: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase2Request {
    pub user_id: String,
    pub message: String,
    pub signing_package: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase2Response {
    pub success: bool,
    pub signature_share: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggregateSignatureRequest {
    pub signature_shares: Vec<String>,
    pub signing_package: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggregateSignatureResponse {
    pub success: bool,
    pub signature: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: u32,
    pub timestamp: String,
    pub key_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterStatus {
    pub total_nodes: usize,
    pub available_nodes: usize,
    pub threshold: usize,
    pub is_operational: bool,
    pub node_statuses: Vec<NodeStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeStatus {
    pub url: String,
    pub available: bool,
    pub node_id: Option<u32>,
    pub last_seen: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct MpcClient {
    client: Client,
    nodes: Vec<String>,
    threshold: u32,
    request_timeout: Duration,
}

impl MpcClient {
    pub fn new(nodes: Vec<String>, threshold: u32) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            nodes,
            threshold,
            request_timeout: Duration::from_secs(30),
        }
    }

    /// Generate distributed keys for a user across all MPC nodes
    pub async fn generate_key(&self, user_id: &Uuid) -> Result<String, MpcError> {
        let user_id_str = user_id.to_string();
        info!("Starting distributed key generation for user {}", user_id_str);

        // Check cluster health first
        let available_nodes = self.check_node_health().await;
        if available_nodes.len() < self.threshold as usize {
            return Err(MpcError::InsufficientNodes {
                available: available_nodes.len(),
                required: self.threshold as usize,
            });
        }

        // Phase 1: Trigger key generation on all available nodes
        let keygen_request = KeyGenRequest {
            user_id: user_id_str.clone(),
            threshold: self.threshold,
            total_parties: available_nodes.len() as u32,
        };

        let mut keygen_futures = Vec::new();
        for node_url in &available_nodes {
            let req = keygen_request.clone();
            let url = node_url.clone();
            let client = self.client.clone();
            let timeout = self.request_timeout;
            
            keygen_futures.push(async move {
                Self::send_keygen_request(&client, &url, &req, timeout).await
            });
        }

        let keygen_results = join_all(keygen_futures).await;
        
        // Collect successful responses
        let mut successful_nodes = Vec::new();
        for (i, result) in keygen_results.iter().enumerate() {
            match result {
                Ok(response) if response.success => {
                    successful_nodes.push(available_nodes[i].clone());
                    info!("Node {} completed key generation", available_nodes[i]);
                }
                Ok(response) => {
                    warn!("Node {} failed key generation: {:?}", available_nodes[i], response.error);
                }
                Err(e) => {
                    warn!("Node {} key generation error: {}", available_nodes[i], e);
                }
            }
        }

        if successful_nodes.len() < self.threshold as usize {
            return Err(MpcError::KeyGenerationFailed(format!(
                "Only {}/{} nodes completed key generation",
                successful_nodes.len(),
                self.threshold
            )));
        }

        // Phase 2: Aggregate public keys from successful nodes
        let aggregate_request = AggregateKeysRequest {
            user_id: user_id_str.clone(),
        };

        let mut aggregate_futures = Vec::new();
        for node_url in &successful_nodes {
            let req = aggregate_request.clone();
            let url = node_url.clone();
            let client = self.client.clone();
            let timeout = self.request_timeout;
            
            aggregate_futures.push(async move {
                Self::send_aggregate_request(&client, &url, &req, timeout).await
            });
        }

        let aggregate_results = join_all(aggregate_futures).await;
        
        // All nodes should return the same aggregated public key
        let mut public_keys = HashMap::new();
        for (i, result) in aggregate_results.iter().enumerate() {
            match result {
                Ok(response) if response.success && response.public_key.is_some() => {
                    let key = response.public_key.as_ref().unwrap();
                    *public_keys.entry(key.clone()).or_insert(0) += 1;
                }
                _ => {
                    warn!("Node {} failed to return aggregated key", successful_nodes[i]);
                }
            }
        }

        // Find the most common public key (should be unanimous)
        let public_key = public_keys
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(key, _)| key)
            .ok_or_else(|| MpcError::AggregationFailed("No consensus on public key".to_string()))?;

        info!("Successfully generated distributed key for user {}: {}", user_id_str, public_key);
        Ok(public_key)
    }

    /// Sign a message using distributed MPC
    pub async fn sign_message(&self, user_id: &Uuid, message_hex: &str) -> Result<String, MpcError> {
        let user_id_str = user_id.to_string();
        info!("Starting distributed signing for user {}", user_id_str);

        // Check cluster health
        let available_nodes = self.check_node_health().await;
        if available_nodes.len() < self.threshold as usize {
            return Err(MpcError::InsufficientNodes {
                available: available_nodes.len(),
                required: self.threshold as usize,
            });
        }

        // Phase 1: Generate nonce commitments from each node
        let phase1_request = SignPhase1Request {
            user_id: user_id_str.clone(),
            message: message_hex.to_string(),
        };

        let mut phase1_futures = Vec::new();
        for node_url in &available_nodes[..self.threshold as usize] {
            let req = phase1_request.clone();
            let url = node_url.clone();
            let client = self.client.clone();
            let timeout = self.request_timeout;
            
            phase1_futures.push(async move {
                Self::send_sign_phase1_request(&client, &url, &req, timeout).await
            });
        }

        let phase1_results = join_all(phase1_futures).await;
        
        // Collect signing packages (should be the same from all nodes)
        let mut signing_packages = HashMap::new();
        let mut successful_phase1_nodes = Vec::new();
        
        for (i, result) in phase1_results.iter().enumerate() {
            match result {
                Ok(response) if response.success && response.signing_package.is_some() => {
                    let package = response.signing_package.as_ref().unwrap();
                    *signing_packages.entry(package.clone()).or_insert(0) += 1;
                    successful_phase1_nodes.push(available_nodes[i].clone());
                }
                _ => {
                    warn!("Node {} failed phase 1 signing", available_nodes[i]);
                }
            }
        }

        if successful_phase1_nodes.len() < self.threshold as usize {
            return Err(MpcError::SigningFailed(format!(
                "Phase 1 failed: only {}/{} nodes succeeded",
                successful_phase1_nodes.len(),
                self.threshold
            )));
        }

        // Get the consensus signing package
        let signing_package = signing_packages
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(package, _)| package)
            .ok_or_else(|| MpcError::SigningFailed("No consensus on signing package".to_string()))?;

        // Phase 2: Generate signature shares
        let phase2_request = SignPhase2Request {
            user_id: user_id_str.clone(),
            message: message_hex.to_string(),
            signing_package: signing_package.clone(),
        };

        let mut phase2_futures = Vec::new();
        for node_url in &successful_phase1_nodes {
            let req = phase2_request.clone();
            let url = node_url.clone();
            let client = self.client.clone();
            let timeout = self.request_timeout;
            
            phase2_futures.push(async move {
                Self::send_sign_phase2_request(&client, &url, &req, timeout).await
            });
        }

        let phase2_results = join_all(phase2_futures).await;
        
        // Collect signature shares
        let mut signature_shares = Vec::new();
        for (i, result) in phase2_results.iter().enumerate() {
            match result {
                Ok(response) if response.success && response.signature_share.is_some() => {
                    signature_shares.push(response.signature_share.as_ref().unwrap().clone());
                }
                _ => {
                    warn!("Node {} failed phase 2 signing", successful_phase1_nodes[i]);
                }
            }
        }

        if signature_shares.len() < self.threshold as usize {
            return Err(MpcError::SigningFailed(format!(
                "Phase 2 failed: only {}/{} signature shares collected",
                signature_shares.len(),
                self.threshold
            )));
        }

        // Aggregate signature shares
        let aggregate_request = AggregateSignatureRequest {
            signature_shares,
            signing_package,
        };

        // Send to any available node for aggregation
        let aggregate_response = Self::send_aggregate_signature_request(
            &self.client,
            &successful_phase1_nodes[0],
            &aggregate_request,
            self.request_timeout,
        ).await?;

        if !aggregate_response.success {
            return Err(MpcError::AggregationFailed(
                aggregate_response.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        let signature = aggregate_response.signature
            .ok_or_else(|| MpcError::SigningFailed("No signature in response".to_string()))?;

        info!("Successfully completed distributed signing for user {}", user_id_str);
        Ok(signature)
    }

    /// Sign a Solana transaction
    pub async fn sign_transaction(
        &self, 
        user_id: &Uuid, 
        tx_hash: &str, 
        _tx_data: &str
    ) -> Result<String, MpcError> {
        // For Solana, we sign the transaction hash
        self.sign_message(user_id, tx_hash).await
    }

    /// Check if threshold number of nodes are available
    pub async fn check_threshold_availability(&self) -> bool {
        let available = self.check_node_health().await.len();
        available >= self.threshold as usize
    }

    /// Get cluster health status
    pub async fn get_cluster_status(&self) -> ClusterStatus {
        let mut node_statuses = Vec::new();
        
        for node_url in &self.nodes {
            let status = match self.check_single_node_health(node_url).await {
                Ok(health) => NodeStatus {
                    url: node_url.clone(),
                    available: true,
                    node_id: Some(health.node_id),
                    last_seen: Some(health.timestamp),
                    error: None,
                },
                Err(e) => NodeStatus {
                    url: node_url.clone(),
                    available: false,
                    node_id: None,
                    last_seen: None,
                    error: Some(e.to_string()),
                },
            };
            node_statuses.push(status);
        }

        let available_count = node_statuses.iter().filter(|s| s.available).count();

        ClusterStatus {
            total_nodes: self.nodes.len(),
            available_nodes: available_count,
            threshold: self.threshold as usize,
            is_operational: available_count >= self.threshold as usize,
            node_statuses,
        }
    }

    // Internal helper methods
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

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        Ok(response.json().await?)
    }

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

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
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

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
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

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
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

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        Ok(response.json().await?)
    }

    async fn check_node_health(&self) -> Vec<String> {
        let mut available_nodes = Vec::new();
        
        for node_url in &self.nodes {
            match self.check_single_node_health(node_url).await {
                Ok(_) => {
                    available_nodes.push(node_url.clone());
                }
                Err(e) => {
                    warn!("Node {} is unavailable: {}", node_url, e);
                }
            }
        }
        
        info!("Available MPC nodes: {}/{}", available_nodes.len(), self.nodes.len());
        available_nodes
    }

    async fn check_single_node_health(&self, node_url: &str) -> Result<HealthResponse, MpcError> {
        let url = format!("{}/health", node_url);
        
        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(MpcError::NodeError(format!("Health check failed: HTTP {}", response.status())));
        }

        Ok(response.json().await?)
    }
}

/// Create default MPC client with environment configuration
pub fn create_default_mpc_client() -> MpcClient {
    let nodes = std::env::var("MPC_NODES")
        .map(|nodes| nodes.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|_| vec![
            "http://localhost:8001".to_string(),
            "http://localhost:8002".to_string(),
            "http://localhost:8003".to_string(),
        ]);

    let threshold = std::env::var("MPC_THRESHOLD")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(2);

    info!("Initializing MPC client with {} nodes, threshold {}", nodes.len(), threshold);
    MpcClient::new(nodes, threshold)
}