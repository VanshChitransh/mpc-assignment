use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn, error};
use futures::future::join_all;
use rand::{seq::SliceRandom, thread_rng};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatus {
    pub url: String,
    pub available: bool,
    pub node_id: Option<u32>,
    pub last_seen: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub total_nodes: usize,
    pub available_nodes: usize,
    pub threshold: u32,
    pub is_operational: bool,
    pub node_statuses: Vec<NodeStatus>,
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

    /// Generate distributed keys for a user across all MPC nodes
    pub async fn generate_key(&self, user_id: &str) -> Result<String, MpcError> {
        let available_nodes = self.check_node_health().await;
        
        if available_nodes.is_empty() {
            return Err(MpcError::AllNodesDown);
        }
        
        if available_nodes.len() < self.threshold as usize {
            return Err(MpcError::InsufficientNodes {
                available: available_nodes.len(),
                required: self.threshold as usize,
            });
        }
        
        info!("Starting key generation for user {} across {} nodes", user_id, available_nodes.len());
        
        // Create key generation request
        let keygen_request = KeyGenRequest {
            user_id: user_id.to_string(),
            threshold: self.threshold,
            total_parties: available_nodes.len() as u32,
        };
        
        // Send request to all available nodes
        let mut futures = Vec::new();
        for node in &available_nodes {
            let client = self.client.clone();
            let request = keygen_request.clone();
            let node_url = node.clone();
            let timeout = self.request_timeout;
            
            futures.push(async move {
                Self::send_keygen_request(&client, &node_url, &request, timeout).await
            });
        }
        
        // Wait for all responses
        let responses = join_all(futures).await;
        
        // Process responses
        let mut public_keys = Vec::new();
        let mut errors = Vec::new();
        
        for (i, result) in responses.into_iter().enumerate() {
            match result {
                Ok(response) if response.success && response.public_key.is_some() => {
                    public_keys.push(response.public_key.unwrap());
                }
                Ok(response) if !response.success => {
                    let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                    errors.push(format!("Node {} failed: {}", available_nodes[i], error_msg));
                }
                Err(e) => {
                    errors.push(format!("Node {} error: {}", available_nodes[i], e));
                }
                _ => {
                    errors.push(format!("Node {} returned unexpected response", available_nodes[i]));
                }
            }
        }
        
        if public_keys.is_empty() {
            return Err(MpcError::KeyGenerationFailed(
                errors.join(", ")
            ));
        }
        
        // All public keys should be the same, just return the first one
        Ok(public_keys[0].clone())
    }

    /// Sign a transaction using MPC nodes
    pub async fn sign_transaction(
        &self,
        user_id: &str,
        message_hash: &str,
        transaction_data: &str
    ) -> Result<String, MpcError> {
        let available_nodes = self.check_node_health().await;
        
        if available_nodes.is_empty() {
            return Err(MpcError::AllNodesDown);
        }
        
        if available_nodes.len() < self.threshold as usize {
            return Err(MpcError::InsufficientNodes {
                available: available_nodes.len(),
                required: self.threshold as usize,
            });
        }
        
        info!("Starting transaction signing for user {}", user_id);
        
        // Create signing request
        let sign_request = SignRequest {
            user_id: user_id.to_string(),
            message_hash: message_hash.to_string(),
            transaction_data: transaction_data.to_string(),
        };
        
        // Randomly select nodes for signing (for better load distribution)
        let mut rng = thread_rng();
        let selected_nodes: Vec<String> = available_nodes
            .choose_multiple(&mut rng, self.threshold as usize)
            .cloned()
            .collect();
            
        // Send request to selected nodes
        let mut futures = Vec::new();
        for node in &selected_nodes {
            let client = self.client.clone();
            let request = sign_request.clone();
            let node_url = node.clone();
            let timeout = self.request_timeout;
            
            futures.push(async move {
                Self::send_sign_request(&client, &node_url, &request, timeout).await
            });
        }
        
        // Wait for all responses
        let responses = join_all(futures).await;
        
        // Process responses
        let mut signatures = Vec::new();
        let mut errors = Vec::new();
        
        for (i, result) in responses.into_iter().enumerate() {
            match result {
                Ok(response) if response.success && response.signature.is_some() => {
                    signatures.push(response.signature.unwrap());
                }
                Ok(response) if !response.success => {
                    let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                    errors.push(format!("Node {} failed: {}", selected_nodes[i], error_msg));
                }
                Err(e) => {
                    errors.push(format!("Node {} error: {}", selected_nodes[i], e));
                }
                _ => {
                    errors.push(format!("Node {} returned unexpected response", selected_nodes[i]));
                }
            }
        }
        
        if signatures.is_empty() {
            return Err(MpcError::SigningFailed(
                errors.join(", ")
            ));
        }
        
        // For now, just return the first signature (in a real implementation we'd aggregate them)
        Ok(signatures[0].clone())
    }
    
    /// Get status of all MPC nodes
    pub async fn get_cluster_status(&self) -> ClusterStatus {
        let available_nodes = self.check_node_health().await;
        
        let mut node_statuses = Vec::new();
        
        // Add status for each node
        for node in &self.nodes {
            let is_available = available_nodes.contains(node);
            
            node_statuses.push(NodeStatus {
                url: node.clone(),
                available: is_available,
                node_id: None, // We don't have node ID info here
                last_seen: if is_available {
                    Some(chrono::Utc::now().to_rfc3339())
                } else {
                    None
                },
                error: None,
            });
        }
        
        ClusterStatus {
            total_nodes: self.nodes.len(),
            available_nodes: available_nodes.len(),
            threshold: self.threshold,
            is_operational: available_nodes.len() >= self.threshold as usize,
            node_statuses,
        }
    }
    
    // Helper Methods for Network Communication
    
    /// Send key generation request to a node
    async fn send_keygen_request(
        client: &Client,
        node_url: &str, 
        request: &KeyGenRequest,
        timeout: Duration
    ) -> Result<KeyGenResponse, MpcError> {
        let response = client
            .post(&format!("{}/api/keygen", node_url))
            .timeout(timeout)
            .json(request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        response.json::<KeyGenResponse>().await
            .map_err(|e| MpcError::RequestFailed(e))
    }

    /// Send signing request to a node
    async fn send_sign_request(
        client: &Client,
        node_url: &str,
        request: &SignRequest,
        timeout: Duration
    ) -> Result<SignResponse, MpcError> {
        let response = client
            .post(&format!("{}/api/sign", node_url))
            .timeout(timeout)
            .json(request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MpcError::NodeError(format!("HTTP {}: {}", status, error_text)));
        }

        response.json::<SignResponse>().await
            .map_err(|e| MpcError::RequestFailed(e))
    }
    
    /// Check health of all nodes
    async fn check_node_health(&self) -> Vec<String> {
        let mut available_nodes = Vec::new();
        
        let mut futures = Vec::new();
        for node in &self.nodes {
            let client = self.client.clone();
            let node_url = node.clone();
            
            futures.push(async move {
                let result = client
                    .get(&format!("{}/health", node_url))
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await;
                
                (node_url, result)
            });
        }
        
        let results = join_all(futures).await;
        
        for (node_url, result) in results {
            match result {
                Ok(response) if response.status().is_success() => {
                    available_nodes.push(node_url);
                }
                _ => {
                    warn!("Node {} is unavailable", node_url);
                }
            }
        }
        
        info!("Available MPC nodes: {}/{}", available_nodes.len(), self.nodes.len());
        available_nodes
    }
}