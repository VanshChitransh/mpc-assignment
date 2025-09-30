//! MPC Client Service for coordinating distributed cryptographic operations
//! 
//! This module provides a client for communicating with multiple MPC nodes
//! to perform threshold signing operations. It includes:
//! - Distributed key generation
//! - Two-phase threshold signing
//! - Health monitoring and node availability checks
//! - Load balancing across MPC nodes
//! - Retry logic with exponential backoff
//! - Circuit breaker pattern for fault tolerance

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum MpcError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTTP error: {status}, body: {body}")]
    Http { status: StatusCode, body: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("MPC node error: {0}")]
    MpcNode(String),

    #[error("Threshold not met: {available} nodes available, need {required}")]
    ThresholdNotMet { available: usize, required: usize },

    #[error("No healthy nodes available")]
    NoHealthyNodes,

    #[error("Key generation failed: {0}")]
    KeygenFailed(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Timeout: operation took longer than {0:?}")]
    Timeout(Duration),

    #[error("Circuit breaker open for node {0}")]
    CircuitBreakerOpen(String),

    #[error("All retries exhausted")]
    AllRetriesExhausted,
}

pub type Result<T> = std::result::Result<T, MpcError>;

// ============================================================================
// Configuration
// ============================================================================

/// Load balancing strategy for selecting MPC nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    /// Simple round-robin selection
    RoundRobin,
    /// Select based on node health scores
    HealthBased,
    /// Random selection
    Random,
}

/// Configuration for MPC client
#[derive(Debug, Clone)]
pub struct MpcConfig {
    /// URLs of MPC nodes (e.g., ["http://localhost:8001", ...])
    pub node_urls: Vec<String>,
    
    /// Timeout for HTTP requests
    pub request_timeout: Duration,
    
    /// Number of retry attempts for failed requests
    pub max_retries: u32,
    
    /// Initial backoff duration for retries
    pub initial_backoff: Duration,
    
    /// Maximum backoff duration for retries
    pub max_backoff: Duration,
    
    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,
    
    /// Threshold for signing (minimum number of nodes required)
    pub signing_threshold: usize,
    
    /// Circuit breaker: max failures before opening circuit
    pub circuit_breaker_threshold: usize,
    
    /// Circuit breaker: time before attempting recovery
    pub circuit_breaker_timeout: Duration,
}

impl Default for MpcConfig {
    fn default() -> Self {
        Self {
            node_urls: vec![
                "http://localhost:8001".to_string(),
                "http://localhost:8002".to_string(),
                "http://localhost:8003".to_string(),
            ],
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            load_balancing: LoadBalancingStrategy::HealthBased,
            signing_threshold: 2,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(30),
        }
    }
}

// ============================================================================
// API Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeygenRequest {
    pub user_id: String,
    pub threshold: usize,
    pub max_participants: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeygenResponse {
    pub user_id: String,
    pub public_key: String,
    pub participant_id: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase1Request {
    pub user_id: String,
    pub message: String,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase1Response {
    pub session_id: String,
    pub participant_id: usize,
    pub commitment: String,
    pub nonce: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase2Request {
    pub user_id: String,
    pub message: String,
    pub session_id: String,
    pub commitments: Vec<CommitmentData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommitmentData {
    pub participant_id: usize,
    pub commitment: String,
    pub nonce: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignPhase2Response {
    pub session_id: String,
    pub participant_id: usize,
    pub signature_share: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggregateRequest {
    pub session_id: String,
    pub signature_shares: Vec<SignatureShare>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignatureShare {
    pub participant_id: usize,
    pub signature_share: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SigningResponse {
    pub signature: String,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: usize,
    pub version: String,
}

// ============================================================================
// Node Health Tracking
// ============================================================================

#[derive(Debug, Clone)]
pub struct NodeHealth {
    pub url: String,
    pub is_healthy: bool,
    pub last_check: Instant,
    pub consecutive_failures: usize,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub average_response_time: Duration,
    pub circuit_breaker_open_until: Option<Instant>,
}

impl NodeHealth {
    fn new(url: String) -> Self {
        Self {
            url,
            is_healthy: true,
            last_check: Instant::now(),
            consecutive_failures: 0,
            total_requests: 0,
            successful_requests: 0,
            average_response_time: Duration::from_millis(0),
            circuit_breaker_open_until: None,
        }
    }

    fn record_success(&mut self, response_time: Duration) {
        self.is_healthy = true;
        self.consecutive_failures = 0;
        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_check = Instant::now();
        
        // Update average response time with exponential moving average
        if self.average_response_time.is_zero() {
            self.average_response_time = response_time;
        } else {
            let alpha = 0.3;
            let current_ms = self.average_response_time.as_millis() as f64;
            let new_ms = response_time.as_millis() as f64;
            let updated_ms = (alpha * new_ms + (1.0 - alpha) * current_ms) as u64;
            self.average_response_time = Duration::from_millis(updated_ms);
        }
    }

    fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.total_requests += 1;
        self.last_check = Instant::now();
        
        if self.consecutive_failures >= 3 {
            self.is_healthy = false;
        }
    }

    fn open_circuit_breaker(&mut self, timeout: Duration) {
        self.circuit_breaker_open_until = Some(Instant::now() + timeout);
        self.is_healthy = false;
        warn!("Circuit breaker opened for node: {}", self.url);
    }

    fn is_circuit_breaker_open(&self) -> bool {
        if let Some(open_until) = self.circuit_breaker_open_until {
            Instant::now() < open_until
        } else {
            false
        }
    }

    fn try_close_circuit_breaker(&mut self) -> bool {
        if let Some(open_until) = self.circuit_breaker_open_until {
            if Instant::now() >= open_until {
                self.circuit_breaker_open_until = None;
                self.consecutive_failures = 0;
                info!("Circuit breaker closed for node: {}", self.url);
                return true;
            }
        }
        false
    }

    fn health_score(&self) -> f64 {
        if !self.is_healthy || self.is_circuit_breaker_open() {
            return 0.0;
        }

        let success_rate = if self.total_requests > 0 {
            self.successful_requests as f64 / self.total_requests as f64
        } else {
            1.0
        };

        // Penalize slow response times
        let response_penalty = if self.average_response_time > Duration::from_secs(1) {
            0.5
        } else {
            1.0
        };

        success_rate * response_penalty
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub threshold_met: bool,
    pub node_details: Vec<NodeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub url: String,
    pub is_healthy: bool,
    pub success_rate: f64,
    pub average_response_ms: u64,
    pub circuit_breaker_open: bool,
}

// ============================================================================
// MPC Client
// ============================================================================

/// MPC Client for coordinating distributed cryptographic operations
pub struct MpcClient {
    config: MpcConfig,
    http_client: Client,
    node_health: Arc<RwLock<HashMap<String, NodeHealth>>>,
    round_robin_index: Arc<RwLock<usize>>,
}

impl MpcClient {
    /// Create a new MPC client with the given configuration
    pub fn new(config: MpcConfig) -> Self {
        let http_client = Client::builder()
            .timeout(config.request_timeout)
            .build()
            .expect("Failed to create HTTP client");

        let mut node_health = HashMap::new();
        for url in &config.node_urls {
            node_health.insert(url.clone(), NodeHealth::new(url.clone()));
        }

        Self {
            config,
            http_client,
            node_health: Arc::new(RwLock::new(node_health)),
            round_robin_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Create a new MPC client with default configuration
    pub fn with_default_config() -> Self {
        Self::new(MpcConfig::default())
    }

    // ========================================================================
    // Node Selection and Health Management
    // ========================================================================

    async fn select_node(&self) -> Result<String> {
        match self.config.load_balancing {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin().await,
            LoadBalancingStrategy::HealthBased => self.select_health_based().await,
            LoadBalancingStrategy::Random => self.select_random().await,
        }
    }

    async fn select_round_robin(&self) -> Result<String> {
        let health = self.node_health.read().await;
        let healthy_nodes: Vec<_> = health
            .values()
            .filter(|n| n.is_healthy && !n.is_circuit_breaker_open())
            .collect();

        if healthy_nodes.is_empty() {
            return Err(MpcError::NoHealthyNodes);
        }

        let mut index = self.round_robin_index.write().await;
        let selected = &healthy_nodes[*index % healthy_nodes.len()];
        *index += 1;

        Ok(selected.url.clone())
    }

    async fn select_health_based(&self) -> Result<String> {
        let health = self.node_health.read().await;
        let mut scored_nodes: Vec<_> = health
            .values()
            .filter(|n| n.is_healthy && !n.is_circuit_breaker_open())
            .map(|n| (n.url.clone(), n.health_score()))
            .collect();

        if scored_nodes.is_empty() {
            return Err(MpcError::NoHealthyNodes);
        }

        scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(scored_nodes[0].0.clone())
    }

    async fn select_random(&self) -> Result<String> {
        use rand::seq::SliceRandom;
        
        let health = self.node_health.read().await;
        let healthy_nodes: Vec<_> = health
            .values()
            .filter(|n| n.is_healthy && !n.is_circuit_breaker_open())
            .map(|n| n.url.clone())
            .collect();

        if healthy_nodes.is_empty() {
            return Err(MpcError::NoHealthyNodes);
        }

        let mut rng = rand::thread_rng();
        Ok(healthy_nodes.choose(&mut rng).unwrap().clone())
    }

    async fn record_success(&self, node_url: &str, response_time: Duration) {
        let mut health = self.node_health.write().await;
        if let Some(node) = health.get_mut(node_url) {
            node.record_success(response_time);
            node.try_close_circuit_breaker();
        }
    }

    async fn record_failure(&self, node_url: &str) {
        let mut health = self.node_health.write().await;
        if let Some(node) = health.get_mut(node_url) {
            node.record_failure();
            
            if node.consecutive_failures >= self.config.circuit_breaker_threshold {
                node.open_circuit_breaker(self.config.circuit_breaker_timeout);
            }
        }
    }

    // ========================================================================
    // HTTP Request Helpers
    // ========================================================================

    async fn post_with_retry<T, R>(
        &self,
        endpoint: &str,
        payload: &T,
    ) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let mut attempts = 0;
        let mut backoff = self.config.initial_backoff;

        loop {
            attempts += 1;

            let node_url = self.select_node().await?;
            let url = format!("{}{}", node_url, endpoint);

            debug!(
                "Attempt {}/{} - Sending request to {}",
                attempts, self.config.max_retries + 1, url
            );

            let start = Instant::now();
            let result = self
                .http_client
                .post(&url)
                .json(payload)
                .send()
                .await;

            match result {
                Ok(response) => {
                    let elapsed = start.elapsed();

                    if response.status().is_success() {
                        self.record_success(&node_url, elapsed).await;

                        match response.json::<R>().await {
                            Ok(data) => {
                                info!("Request succeeded in {:?}", elapsed);
                                return Ok(data);
                            }
                            Err(e) => {
                                error!("Failed to parse response: {}", e);
                                self.record_failure(&node_url).await;
                                
                                if attempts > self.config.max_retries {
                                    return Err(MpcError::InvalidResponse(e.to_string()));
                                }
                            }
                        }
                    } else {
                        let status = response.status();
                        let body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unable to read body".to_string());

                        error!("HTTP error: {} - {}", status, body);
                        self.record_failure(&node_url).await;

                        if attempts > self.config.max_retries {
                            return Err(MpcError::Http { status, body });
                        }
                    }
                }
                Err(e) => {
                    error!("Network error: {}", e);
                    self.record_failure(&node_url).await;

                    if attempts > self.config.max_retries {
                        return Err(MpcError::Network(e));
                    }
                }
            }

            // Exponential backoff
            tokio::time::sleep(backoff).await;
            backoff = std::cmp::min(backoff * 2, self.config.max_backoff);
        }
    }
    #[allow(dead_code)]
    async fn get_with_retry<R>(&self, node_url: &str, endpoint: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", node_url, endpoint);
        let mut attempts = 0;
        let mut backoff = self.config.initial_backoff;

        loop {
            attempts += 1;

            let start = Instant::now();
            let result = self.http_client.get(&url).send().await;

            match result {
                Ok(response) => {
                    let elapsed = start.elapsed();

                    if response.status().is_success() {
                        self.record_success(node_url, elapsed).await;

                        match response.json::<R>().await {
                            Ok(data) => return Ok(data),
                            Err(e) => {
                                if attempts > self.config.max_retries {
                                    return Err(MpcError::InvalidResponse(e.to_string()));
                                }
                            }
                        }
                    } else {
                        self.record_failure(node_url).await;

                        if attempts > self.config.max_retries {
                            let status = response.status();
                            let body = response.text().await.unwrap_or_default();
                            return Err(MpcError::Http { status, body });
                        }
                    }
                }
                Err(e) => {
                    self.record_failure(node_url).await;

                    if attempts > self.config.max_retries {
                        return Err(MpcError::Network(e));
                    }
                }
            }

            tokio::time::sleep(backoff).await;
            backoff = std::cmp::min(backoff * 2, self.config.max_backoff);
        }
    }

    // ========================================================================
    // Public API - Core MPC Operations
    // ========================================================================

    /// Generate a distributed key for a user
    /// 
    /// This coordinates key generation across all MPC nodes and returns
    /// the aggregated public key that can be used as the user's wallet address.
    pub async fn generate_key(&self, user_id: &str) -> Result<String> {
        info!("Starting distributed key generation for user: {}", user_id);

        let request = KeygenRequest {
            user_id: user_id.to_string(),
            threshold: self.config.signing_threshold,
            max_participants: self.config.node_urls.len(),
        };

        // Send keygen request to all nodes
        let mut handles = vec![];
        for node_url in &self.config.node_urls {
            let client = self.http_client.clone();
            let url = format!("{}/api/keygen", node_url);
            let req = request.clone();
            let node_url_clone = node_url.clone();
            let node_health = self.node_health.clone();
            let _circuit_breaker_timeout = self.config.circuit_breaker_timeout;

            let handle = tokio::spawn(async move {
                let start = Instant::now();
                let result = client
                    .post(&url)
                    .json(&req)
                    .send()
                    .await;

                match result {
                    Ok(response) if response.status().is_success() => {
                        let elapsed = start.elapsed();
                        match response.json::<KeygenResponse>().await {
                            Ok(data) => {
                                // Record success
                                let mut health = node_health.write().await;
                                if let Some(node) = health.get_mut(&node_url_clone) {
                                    node.record_success(elapsed);
                                    node.try_close_circuit_breaker();
                                }
                                Ok(data)
                            }
                            Err(e) => Err(MpcError::InvalidResponse(e.to_string())),
                        }
                    }
                    Ok(response) => {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        Err(MpcError::Http { status, body })
                    }
                    Err(e) => Err(MpcError::Network(e)),
                }
            });

            handles.push(handle);
        }

        // Wait for all responses
        let results = futures::future::join_all(handles).await;
        
        let mut responses = Vec::new();
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(Ok(response)) => responses.push(response),
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(MpcError::MpcNode(format!("Task error: {}", e))),
            }
        }

        if responses.is_empty() {
            return Err(MpcError::KeygenFailed(format!(
                "All nodes failed. Errors: {:?}",
                errors
            )));
        }

        if responses.len() < self.config.signing_threshold {
            return Err(MpcError::ThresholdNotMet {
                available: responses.len(),
                required: self.config.signing_threshold,
            });
        }

        // All responses should have the same public key
        let public_key = responses[0].public_key.clone();
        for response in &responses[1..] {
            if response.public_key != public_key {
                return Err(MpcError::KeygenFailed(
                    "Inconsistent public keys from nodes".to_string(),
                ));
            }
        }

        info!(
            "Key generation successful for user: {} - Public key: {}",
            user_id, public_key
        );

        Ok(public_key)
    }

    /// Sign a message using distributed threshold signing
    /// 
    /// This implements a two-phase signing protocol:
    /// 1. Phase 1: Collect commitments from all nodes
    /// 2. Phase 2: Collect signature shares and aggregate
    pub async fn sign_message(&self, user_id: &str, message_hex: &str) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        info!(
            "Starting distributed signing for user: {} (session: {})",
            user_id, session_id
        );

        // Phase 1: Collect commitments
        let commitments = self.signing_phase1(user_id, message_hex, &session_id).await?;

        if commitments.len() < self.config.signing_threshold {
            return Err(MpcError::ThresholdNotMet {
                available: commitments.len(),
                required: self.config.signing_threshold,
            });
        }

        // Phase 2: Collect signature shares
        let signature_shares = self
            .signing_phase2(user_id, message_hex, &session_id, &commitments)
            .await?;

        if signature_shares.len() < self.config.signing_threshold {
            return Err(MpcError::ThresholdNotMet {
                available: signature_shares.len(),
                required: self.config.signing_threshold,
            });
        }

        // Aggregate signature shares
        let signature = self.aggregate_signature(&session_id, &signature_shares).await?;

        info!(
            "Signing successful for user: {} (session: {})",
            user_id, session_id
        );

        Ok(signature)
    }

    /// Sign a Solana transaction using distributed threshold signing
    /// 
    /// This is a convenience method that handles transaction-specific formatting
    pub async fn sign_transaction(
        &self,
        user_id: &str,
        transaction_hash: &str,
        _transaction_data: &str,
    ) -> Result<String> {
        info!(
            "Signing transaction for user: {} (hash: {})",
            user_id, transaction_hash
        );

        self.sign_message(user_id, transaction_hash).await
    }

    // ========================================================================
    // Internal Signing Protocol
    // ========================================================================

    async fn signing_phase1(
        &self,
        user_id: &str,
        message: &str,
        session_id: &str,
    ) -> Result<Vec<CommitmentData>> {
        debug!("Phase 1: Collecting commitments (session: {})", session_id);

        let request = SignPhase1Request {
            user_id: user_id.to_string(),
            message: message.to_string(),
            session_id: session_id.to_string(),
        };

        let mut handles = vec![];
        for node_url in &self.config.node_urls {
            let client = self.http_client.clone();
            let url = format!("{}/api/sign-phase1", node_url);
            let req = request.clone();

            let handle = tokio::spawn(async move {
                client
                    .post(&url)
                    .json(&req)
                    .send()
                    .await?
                    .json::<SignPhase1Response>()
                    .await
                    .map_err(MpcError::Network)
            });

            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;

        let commitments: Vec<CommitmentData> = results
            .into_iter()
            .filter_map(|r| r.ok().and_then(|rr| rr.ok()))
            .map(|resp| CommitmentData {
                participant_id: resp.participant_id,
                commitment: resp.commitment,
                nonce: resp.nonce,
            })
            .collect();

        debug!("Phase 1 complete: {} commitments collected", commitments.len());

        Ok(commitments)
    }

    async fn signing_phase2(
        &self,
        user_id: &str,
        message: &str,
        session_id: &str,
        commitments: &[CommitmentData],
    ) -> Result<Vec<SignatureShare>> {
        debug!("Phase 2: Collecting signature shares (session: {})", session_id);

        let request = SignPhase2Request {
            user_id: user_id.to_string(),
            message: message.to_string(),
            session_id: session_id.to_string(),
            commitments: commitments.to_vec(),
        };

        let mut handles = vec![];
        for node_url in &self.config.node_urls {
            let client = self.http_client.clone();
            let url = format!("{}/api/sign-phase2", node_url);
            let req = request.clone();

            let handle = tokio::spawn(async move {
                client
                    .post(&url)
                    .json(&req)
                    .send()
                    .await?
                    .json::<SignPhase2Response>()
                    .await
                    .map_err(MpcError::Network)
            });

            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;

        let signature_shares: Vec<SignatureShare> = results
            .into_iter()
            .filter_map(|r| r.ok().and_then(|rr| rr.ok()))
            .map(|resp| SignatureShare {
                participant_id: resp.participant_id,
                signature_share: resp.signature_share,
            })
            .collect();

        debug!(
            "Phase 2 complete: {} signature shares collected",
            signature_shares.len()
        );

        Ok(signature_shares)
    }

    async fn aggregate_signature(
        &self,
        session_id: &str,
        signature_shares: &[SignatureShare],
    ) -> Result<String> {
        debug!("Aggregating signature shares (session: {})", session_id);

        let request = AggregateRequest {
            session_id: session_id.to_string(),
            signature_shares: signature_shares.to_vec(),
        };

        // Use any healthy node for aggregation
        let response: SigningResponse = self.post_with_retry("/api/aggregate", &request).await?;

        Ok(response.signature)
    }

    // ========================================================================
    // Public API - Health and Monitoring
    // ========================================================================

    /// Check the health of all MPC nodes
    pub async fn health_check(&self) -> Result<ClusterStatus> {
        debug!("Performing cluster health check");

        let mut handles = vec![];
        for node_url in &self.config.node_urls {
            let client = self.http_client.clone();
            let url = format!("{}/health", node_url);
            let node_url_clone = node_url.clone();

            let handle = tokio::spawn(async move {
                let result = client.get(&url).send().await;

                match result {
                    Ok(response) if response.status().is_success() => {
                        match response.json::<HealthResponse>().await {
                            Ok(_) => (node_url_clone, true),
                            Err(_) => (node_url_clone, false),
                        }
                    }
                    _ => (node_url_clone, false),
                }
            });

            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;

        // Update health status
        for result in results {
            if let Ok((node_url, is_healthy)) = result {
                let mut health = self.node_health.write().await;
                if let Some(node) = health.get_mut(&node_url) {
                    if is_healthy {
                        node.record_success(Duration::from_millis(100));
                    } else {
                        node.record_failure();
                    }
                }
            }
        }

        self.get_cluster_status().await
    }

    /// Get the current cluster status
    pub async fn get_cluster_status(&self) -> Result<ClusterStatus> {
        let health = self.node_health.read().await;

        let node_details: Vec<NodeStatus> = health
            .values()
            .map(|node| {
                let success_rate = if node.total_requests > 0 {
                    node.successful_requests as f64 / node.total_requests as f64
                } else {
                    0.0
                };

                NodeStatus {
                    url: node.url.clone(),
                    is_healthy: node.is_healthy,
                    success_rate,
                    average_response_ms: node.average_response_time.as_millis() as u64,
                    circuit_breaker_open: node.is_circuit_breaker_open(),
                }
            })
            .collect();

        let healthy_count = node_details.iter().filter(|n| n.is_healthy).count();
        let threshold_met = healthy_count >= self.config.signing_threshold;

        Ok(ClusterStatus {
            total_nodes: self.config.node_urls.len(),
            healthy_nodes: healthy_count,
            threshold_met,
            node_details,
        })
    }

    /// Check if the cluster can meet the signing threshold
    pub async fn check_threshold_availability(&self) -> bool {
        match self.get_cluster_status().await {
            Ok(status) => status.threshold_met,
            Err(_) => false,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MpcConfig::default();
        assert_eq!(config.node_urls.len(), 3);
        assert_eq!(config.signing_threshold, 2);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_node_health_initialization() {
        let health = NodeHealth::new("http://localhost:8001".to_string());
        assert!(health.is_healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.total_requests, 0);
    }

    #[test]
    fn test_node_health_success_recording() {
        let mut health = NodeHealth::new("http://localhost:8001".to_string());
        health.record_success(Duration::from_millis(100));
        
        assert!(health.is_healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.total_requests, 1);
        assert_eq!(health.successful_requests, 1);
    }

    #[test]
    fn test_node_health_failure_recording() {
        let mut health = NodeHealth::new("http://localhost:8001".to_string());
        
        health.record_failure();
        assert!(health.is_healthy); // Still healthy after 1 failure
        
        health.record_failure();
        assert!(health.is_healthy); // Still healthy after 2 failures
        
        health.record_failure();
        assert!(!health.is_healthy); // Unhealthy after 3 consecutive failures
    }

    #[test]
    fn test_circuit_breaker() {
        let mut health = NodeHealth::new("http://localhost:8001".to_string());
        
        health.open_circuit_breaker(Duration::from_millis(100));
        assert!(health.is_circuit_breaker_open());
        assert!(!health.is_healthy);
        
        std::thread::sleep(Duration::from_millis(150));
        assert!(health.try_close_circuit_breaker());
        assert!(!health.is_circuit_breaker_open());
    }
    #[test]
    fn test_health_score_calculation() {
        let mut health = NodeHealth::new("http://localhost:8001".to_string());
        
        // New node is healthy with no track record - gives benefit of doubt with score 1.0
        assert_eq!(health.health_score(), 1.0);
        
        // Good performance after successes
        health.record_success(Duration::from_millis(100));
        health.record_success(Duration::from_millis(100));
        assert!(health.health_score() > 0.9);
        
        // After failures, score decreases
        health.record_failure();
        let score_after_failure = health.health_score();
        assert!(score_after_failure < 1.0);
    }
}