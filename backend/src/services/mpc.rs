use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info, warn};
use uuid::Uuid;
use std::time::Duration;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: u32,
    pub timestamp: String,
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
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            nodes,
            threshold,
            request_timeout: Duration::from_secs(30),
        }
    }

    pub async fn generate_key(&self, user_id: &Uuid) -> Result<String, MpcError> {
        let user_id_str = user_id.to_string();
        info!("Starting key generation for user: {}", user_id_str);

        // First check which nodes are available
        let available_nodes = self.check_node_health().await;
        if available_nodes.len() < self.threshold as usize {
            return Err(MpcError::InsufficientNodes {
                available: available_nodes.len(),
                required: self.threshold as usize,
            });
        }

        let request = KeyGenRequest {
            user_id: user_id_str.clone(),
            threshold: self.threshold,
            total_parties: self.nodes.len() as u32,
        };

        // Send key generation request to all available nodes
        let mut successful_responses = 0;
        let mut errors = Vec::new();

        for node_url in &available_nodes {
            match self.send_keygen_request(node_url, &request).await {
                Ok(response) => {
                    if response.success {
                        successful_responses += 1;
                        info!("Key generation successful on node: {}", node_url);
                    } else {
                        let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                        error!("Key generation failed on node {}: {}", node_url, error_msg);
                        errors.push(format!("{}: {}", node_url, error_msg));
                    }
                }
                Err(e) => {
                    error!("Failed to send request to node {}: {}", node_url, e);
                    errors.push(format!("{}: {}", node_url, e));
                }
            }
        }

        // Check if we have enough successful responses
        if successful_responses < self.threshold {
            return Err(MpcError::KeyGenerationFailed(format!(
                "Only {} out of {} required nodes succeeded. Errors: {}",
                successful_responses,
                self.threshold,
                errors.join(", ")
            )));
        }

        // Wait a moment for key generation to complete across nodes
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Now aggregate the public keys
        self.aggregate_public_keys(&user_id_str).await
    }

    async fn send_keygen_request(&self, node_url: &str, request: &KeyGenRequest) -> Result<KeyGenResponse, MpcError> {
        let url = format!("{}/generate", node_url);
        
        let response = self.client
            .post(&url)
            .timeout(self.request_timeout)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status(); // save status first
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        let response: KeyGenResponse = response.json().await?;
        Ok(response)
    }

    async fn aggregate_public_keys(&self, user_id: &str) -> Result<String, MpcError> {
        info!("Aggregating public keys for user: {}", user_id);

        let request = AggregateKeysRequest {
            user_id: user_id.to_string(),
        };

        let available_nodes = self.check_node_health().await;
        if available_nodes.is_empty() {
            return Err(MpcError::AllNodesDown);
        }

        // Try each node until we get a successful response
        let mut errors = Vec::new();

        for node_url in &available_nodes {
            match self.send_aggregate_request(node_url, &request).await {
                Ok(response) => {
                    if response.success {
                        if let Some(public_key) = response.public_key {
                            info!("Successfully retrieved public key from node: {}", node_url);
                            return Ok(public_key);
                        } else {
                            warn!("Node {} returned success but no public key", node_url);
                        }
                    } else {
                        let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                        warn!("Node {} returned error: {}", node_url, error_msg);
                        errors.push(format!("{}: {}", node_url, error_msg));
                    }
                }
                Err(e) => {
                    error!("Failed to get public key from node {}: {}", node_url, e);
                    errors.push(format!("{}: {}", node_url, e));
                }
            }
        }

        Err(MpcError::KeyGenerationFailed(format!(
            "Failed to retrieve public key from any node. Errors: {}",
            errors.join(", ")
        )))
    }

    async fn send_aggregate_request(&self, node_url: &str, request: &AggregateKeysRequest) -> Result<AggregateKeysResponse, MpcError> {
        let url = format!("{}/aggregate-keys", node_url);
        
        let response = self.client
            .post(&url)
            .timeout(self.request_timeout)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status(); // save status first
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        let response: AggregateKeysResponse = response.json().await?;
        Ok(response)
    }

    pub async fn sign_transaction(
        &self,
        user_id: &Uuid,
        message_hash: &str,
        transaction_data: &str,
    ) -> Result<String, MpcError> {
        let user_id_str = user_id.to_string();
        info!("Starting transaction signing for user: {}", user_id_str);

        let available_nodes = self.check_node_health().await;
        if available_nodes.len() < self.threshold as usize {
            return Err(MpcError::InsufficientNodes {
                available: available_nodes.len(),
                required: self.threshold as usize,
            });
        }

        let request = SignRequest {
            user_id: user_id_str.clone(),
            message_hash: message_hash.to_string(),
            transaction_data: transaction_data.to_string(),
        };

        // Phase 1: Send signing step 1 to all nodes
        let mut step1_successes = 0;
        let mut errors = Vec::new();

        for node_url in &available_nodes {
            match self.send_sign_step1_request(node_url, &request).await {
                Ok(response) => {
                    if response.success {
                        step1_successes += 1;
                        info!("Signing step 1 successful on node: {}", node_url);
                    } else {
                        let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                        error!("Signing step 1 failed on node {}: {}", node_url, error_msg);
                        errors.push(format!("Step1 {}: {}", node_url, error_msg));
                    }
                }
                Err(e) => {
                    error!("Failed to send step1 request to node {}: {}", node_url, e);
                    errors.push(format!("Step1 {}: {}", node_url, e));
                }
            }
        }

        if step1_successes < self.threshold {
            return Err(MpcError::SigningFailed(format!(
                "Step1: Only {} out of {} required nodes succeeded. Errors: {}",
                step1_successes,
                self.threshold,
                errors.join(", ")
            )));
        }

        // Small delay to allow nodes to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Phase 2: Complete signing with step 2
        for node_url in &available_nodes {
            match self.send_sign_step2_request(node_url, &request).await {
                Ok(response) => {
                    if response.success {
                        if let Some(signature) = response.signature {
                            info!("Signing completed successfully on node: {}", node_url);
                            return Ok(signature);
                        }
                    } else {
                        let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                        warn!("Signing step 2 failed on node {}: {}", node_url, error_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to send step2 request to node {}: {}", node_url, e);
                }
            }
        }

        Err(MpcError::SigningFailed(format!(
            "Step2: No valid signature received. Errors: {}",
            errors.join(", ")
        )))
    }

    async fn send_sign_step1_request(&self, node_url: &str, request: &SignRequest) -> Result<SignResponse, MpcError> {
        let url = format!("{}/agg-send-step1", node_url);
        
        let response = self.client
            .post(&url)
            .timeout(self.request_timeout)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status(); // save status first
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        let response: SignResponse = response.json().await?;
        Ok(response)
    }

    async fn send_sign_step2_request(&self, node_url: &str, request: &SignRequest) -> Result<SignResponse, MpcError> {
        let url = format!("{}/agg-send-step2", node_url);
        
        let response = self.client
            .post(&url)
            .timeout(self.request_timeout)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status(); // save status first
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        let response: SignResponse = response.json().await?;
        Ok(response)
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
            .timeout(Duration::from_secs(5)) // Shorter timeout for health checks
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(MpcError::NodeError(format!("Health check failed: HTTP {}", response.status())));
        }

        let health_response: HealthResponse = response.json().await?;
        Ok(health_response)
    }

    /// Get the number of available nodes
    pub async fn get_available_node_count(&self) -> usize {
        self.check_node_health().await.len()
    }

    /// Check if enough nodes are available for operations
    pub async fn is_ready(&self) -> bool {
        let available = self.check_node_health().await.len();
        available >= self.threshold as usize
    }

    /// Get cluster status for monitoring
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
        .unwrap_or_else(|_| "2".to_string())
        .parse()
        .unwrap_or(2);
    
    MpcClient::new(nodes, threshold)
}