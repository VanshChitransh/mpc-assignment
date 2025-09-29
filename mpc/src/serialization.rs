use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Key share stored for each user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShare {
    pub user_id: Uuid,
    pub node_id: u32,
    pub participant_id: [u8; 2], // FROST participant identifier
    pub key_package: Vec<u8>,    // Serialized FROST KeyPackage
    pub public_key: String,      // Hex-encoded public key
    pub threshold: u16,
    pub total_parties: u16,
    pub created_at: DateTime<Utc>,
}

/// Signing session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningState {
    pub session_id: String,
    pub user_id: Uuid,
    pub message: Vec<u8>,
    pub nonces: Vec<u8>,         // Serialized SigningNonces
    pub commitments: Vec<u8>,    // Serialized SigningCommitments
    pub signature_share: Option<Vec<u8>>, // Serialized signature share
    pub created_at: DateTime<Utc>,
}

/// Message types for node-to-node communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MpcMessage {
    KeyGenRound1 {
        session_id: String,
        user_id: Uuid,
        sender_id: u32,
        commitment: Vec<u8>,
    },
    KeyGenRound2 {
        session_id: String,
        user_id: Uuid,
        sender_id: u32,
        shares: Vec<u8>,
    },
    SigningRound1 {
        session_id: String,
        user_id: Uuid,
        sender_id: u32,
        commitments: Vec<u8>,
    },
    SigningRound2 {
        session_id: String,
        user_id: Uuid,
        sender_id: u32,
        signature_share: Vec<u8>,
    },
    Heartbeat {
        sender_id: u32,
        timestamp: DateTime<Utc>,
    },
}

/// Network protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub id: String,
    pub sender: u32,
    pub recipient: Option<u32>, // None for broadcast
    pub message_type: String,
    pub payload: MpcMessage,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<String>, // Message authentication
}

/// Key generation session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyGenSession {
    pub session_id: String,
    pub user_id: Uuid,
    pub threshold: u16,
    pub total_parties: u16,
    pub participants: Vec<u32>,
    pub round: u8,
    pub commitments: std::collections::BTreeMap<u32, Vec<u8>>,
    pub shares: std::collections::BTreeMap<u32, Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Configuration for MPC node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: u32,
    pub bind_address: String,
    pub data_dir: String,
    pub peer_nodes: Vec<String>,
    pub max_concurrent_sessions: usize,
    pub session_timeout_seconds: u64,
    pub heartbeat_interval_seconds: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            bind_address: "127.0.0.1:8001".to_string(),
            data_dir: "./data/node1".to_string(),
            peer_nodes: vec![
                "http://localhost:8002".to_string(),
                "http://localhost:8003".to_string(),
            ],
            max_concurrent_sessions: 100,
            session_timeout_seconds: 300, // 5 minutes
            heartbeat_interval_seconds: 30,
        }
    }
}

/// Database key constants
pub mod db_keys {
    pub const USER_KEY_PREFIX: &str = "user:";
    pub const SESSION_KEY_PREFIX: &str = "session:";
    pub const CONFIG_KEY: &str = "config";
    pub const PEER_NODES_KEY: &str = "peers";
    
    pub fn user_key(user_id: &uuid::Uuid) -> String {
        format!("user:{}", user_id)
    }
    
    pub fn session_key(session_id: &str) -> String {
        format!("session:{}", session_id)
    }
}

/// Utility functions for serialization
impl KeyShare {
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

impl SigningState {
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
    
    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = Utc::now();
        let timeout_duration = chrono::Duration::seconds(timeout_seconds as i64);
        now > self.created_at + timeout_duration
    }
}

impl NetworkMessage {
    pub fn new(sender: u32, payload: MpcMessage) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            sender,
            recipient: None,
            message_type: std::any::type_name::<MpcMessage>().to_string(),
            payload,
            timestamp: Utc::now(),
            signature: None,
        }
    }
    
    pub fn with_recipient(mut self, recipient: u32) -> Self {
        self.recipient = Some(recipient);
        self
    }
    
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }
    
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

impl KeyGenSession {
    pub fn new(
        user_id: Uuid,
        threshold: u16,
        total_parties: u16,
        participants: Vec<u32>,
        timeout_seconds: u64,
    ) -> Self {
        let now = Utc::now();
        let timeout_duration = chrono::Duration::seconds(timeout_seconds as i64);
        
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            threshold,
            total_parties,
            participants,
            round: 1,
            commitments: std::collections::BTreeMap::new(),
            shares: std::collections::BTreeMap::new(),
            created_at: now,
            expires_at: now + timeout_duration,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    pub fn is_complete(&self) -> bool {
        self.commitments.len() >= self.threshold as usize &&
        self.shares.len() >= self.threshold as usize
    }
    
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}