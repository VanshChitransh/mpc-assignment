use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// FROST-compatible key share structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostKeyShare {
    pub user_id: Uuid,
    pub node_id: u32,
    pub identifier: u16,                    // FROST participant identifier (1-indexed)
    pub signing_share: Vec<u8>,             // Serialized SecretShare
    pub verifying_share: Vec<u8>,            // Serialized VerifyingShare
    pub verifying_key: Vec<u8>,             // Group public key
    pub threshold: u16,
    pub max_signers: u16,
    pub created_at: DateTime<Utc>,
}

/// FROST Round 1 state for nonce generation and commitments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostRound1State {
    pub session_id: String,
    pub user_id: Uuid,
    pub message: Vec<u8>,
    pub signing_nonces: Vec<u8>,            // Serialized SigningNonces (secret)
    pub signing_commitments: Vec<u8>,       // Serialized SigningCommitments (public)
    pub created_at: DateTime<Utc>,
}

/// FROST Round 2 state for signature share generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostRound2State {
    pub session_id: String,
    pub signature_share: Vec<u8>,           // Serialized SignatureShare
    pub created_at: DateTime<Utc>,
}

/// Legacy structures for backward compatibility
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

/// Signing session state for FROST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningState {
    pub session_id: String,
    pub user_id: Uuid,
    pub message: Vec<u8>,
    pub nonces: Vec<u8>,         // Serialized FROST SigningNonces
    pub commitments: Vec<u8>,    // Serialized FROST SigningCommitments
    pub signature_share: Option<Vec<u8>>, // Serialized FROST signature share
    pub created_at: DateTime<Utc>,
}

impl SigningState {
    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = Utc::now();
        let timeout_duration = chrono::Duration::seconds(timeout_seconds as i64);
        now > self.created_at + timeout_duration
    }
}

/// Helper functions for FROST serialization
impl FrostKeyShare {
    /// Serialize the key share to bytes for storage
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(bincode::serialize(self)?)
    }
    
    /// Deserialize key share from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(data)?)
    }
}

impl FrostRound1State {
    /// Serialize the round 1 state to bytes
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(bincode::serialize(self)?)
    }
    
    /// Deserialize round 1 state from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(data)?)
    }
}

impl FrostRound2State {
    /// Serialize the round 2 state to bytes
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(bincode::serialize(self)?)
    }
    
    /// Deserialize round 2 state from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(data)?)
    }
}
