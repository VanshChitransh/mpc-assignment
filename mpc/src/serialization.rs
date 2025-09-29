use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Key share stored for each user using FROST
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
