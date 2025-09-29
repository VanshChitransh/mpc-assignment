use crate::error::MpcError;
use crate::serialization::{KeyShare, SigningState};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};
use rand::rngs::OsRng;
use sled::Db;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn, debug};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ThresholdSigningService {
    pub node_id: u32,
    pub db: Arc<Db>,
    pub peer_nodes: Vec<String>,
    pub signing_sessions: Arc<RwLock<BTreeMap<String, SigningState>>>,
    pub client: reqwest::Client,
}

impl ThresholdSigningService {
    pub async fn new(
        node_id: u32, 
        data_dir: &str, 
        peer_nodes: Vec<String>
    ) -> Result<Self, MpcError> {
        let db_path = format!("{}/keys.db", data_dir);
        let db = sled::open(&db_path)
            .map_err(|e| MpcError::StorageError(format!("Failed to open database: {}", e)))?;

        info!("Opened key database at: {}", db_path);

        Ok(Self {
            node_id,
            db: Arc::new(db),
            peer_nodes,
            signing_sessions: Arc::new(RwLock::new(BTreeMap::new())),
            client: reqwest::Client::new(),
        })
    }

    /// Generate a key share for the given user (simplified - not true threshold)
    /// In production, this would use proper FROST distributed key generation
    pub async fn generate_key_share(
        &self,
        user_id: &Uuid,
        threshold: u16,
        total_parties: u16,
    ) -> Result<(), MpcError> {
        let user_key = format!("user:{}", user_id);
        
        info!("Generating key share for user: {} (threshold: {}, total: {})", 
              user_id, threshold, total_parties);

        // Check if key already exists
        if self.db.contains_key(&user_key)? {
            warn!("Key already exists for user: {}", user_id);
            return Ok(()); // Don't error if key already exists
        }

        // Generate a simple Ed25519 keypair (simplified approach)
        // In a real threshold scheme, this would be distributed key generation
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let public_key_bytes = verifying_key.as_bytes();
        let secret_key_bytes = signing_key.as_bytes();

        // Create key share (simplified - real threshold would have polynomial shares)
        let key_share_data = KeyShare {
            user_id: *user_id,
            node_id: self.node_id,
            participant_id: [self.node_id as u8, 0], // Simplified participant ID
            key_package: secret_key_bytes.to_vec(),
            public_key: hex::encode(public_key_bytes),
            threshold,
            total_parties,
            created_at: chrono::Utc::now(),
        };

        let serialized = rmp_serde::to_vec(&key_share_data)
            .map_err(|e| MpcError::SerializationError(format!("Failed to serialize key share: {}", e)))?;

        self.db.insert(&user_key, serialized)?;
        self.db.flush()?;

        info!("Successfully generated and stored key share for user: {}", user_id);
        Ok(())
    }

    /// Get the public key for a user
    pub async fn get_public_key(&self, user_id: &Uuid) -> Result<Option<String>, MpcError> {
        let user_key = format!("user:{}", user_id);
        
        debug!("Retrieving public key for user: {}", user_id);

        let key_data = match self.db.get(&user_key)? {
            Some(data) => data,
            None => {
                debug!("No key found for user: {}", user_id);
                return Ok(None);
            }
        };

        let key_share: KeyShare = rmp_serde::from_slice(&key_data)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize key share: {}", e)))?;

        debug!("Found public key for user: {}", user_id);
        Ok(Some(key_share.public_key))
    }

    /// Prepare for signing (step 1)
    pub async fn prepare_signing(&self, user_id: &Uuid, message_hash: &str) -> Result<(), MpcError> {
        let session_id = format!("{}:{}", user_id, message_hash);
        
        info!("Preparing signing session: {}", session_id);

        // Check if key exists
        let user_key = format!("user:{}", user_id);
        if !self.db.contains_key(&user_key)? {
            return Err(MpcError::KeyNotFound(format!("No key found for user: {}", user_id)));
        }

        // Create signing state
        let signing_state = SigningState {
            session_id: session_id.clone(),
            user_id: *user_id,
            message: hex::decode(message_hash)
                .map_err(|_| MpcError::SerializationError("Invalid message hash hex".to_string()))?,
            nonces: vec![], // Simplified - would contain FROST nonces
            commitments: vec![], // Simplified - would contain FROST commitments
            signature_share: None,
            created_at: chrono::Utc::now(),
        };

        let mut sessions = self.signing_sessions.write().await;
        sessions.insert(session_id.clone(), signing_state);

        info!("Signing session prepared: {}", session_id);
        Ok(())
    }

    /// Sign a message (step 2 - simplified)
    pub async fn sign_message(&self, user_id: &Uuid, message_hash: &str) -> Result<String, MpcError> {
        info!("Signing message for user: {}", user_id);

        // Load key share
        let user_key = format!("user:{}", user_id);
        let key_data = self.db.get(&user_key)?
            .ok_or_else(|| MpcError::KeyNotFound(format!("No key found for user: {}", user_id)))?;

        let key_share: KeyShare = rmp_serde::from_slice(&key_data)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize key share: {}", e)))?;

        // Reconstruct signing key (simplified - real threshold would combine shares)
        let secret_key_bytes: [u8; 32] = key_share.key_package[..32].try_into()
            .map_err(|_| MpcError::CryptographicError("Invalid key length".to_string()))?;
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);

        // Decode message hash
        let message_bytes = hex::decode(message_hash)
            .map_err(|_| MpcError::SerializationError("Invalid message hash hex".to_string()))?;

        // Sign the message
        let signature = signing_key.sign(&message_bytes);
        let signature_hex = hex::encode(signature.to_bytes());

        // Clean up session
        let session_id = format!("{}:{}", user_id, message_hash);
        let mut sessions = self.signing_sessions.write().await;
        sessions.remove(&session_id);

        info!("Message signed successfully for user: {}", user_id);
        Ok(signature_hex)
    }

    /// List all stored user keys (for debugging)
    pub async fn list_user_keys(&self) -> Result<Vec<Uuid>, MpcError> {
        let mut user_ids = Vec::new();
        
        for item in self.db.iter() {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            
            if key_str.starts_with("user:") {
                if let Ok(user_id) = Uuid::parse_str(&key_str[5..]) {
                    user_ids.push(user_id);
                }
            }
        }
        
        Ok(user_ids)
    }

    /// Delete a user's key share (for cleanup)
    pub async fn delete_user_key(&self, user_id: &Uuid) -> Result<bool, MpcError> {
        let user_key = format!("user:{}", user_id);
        
        match self.db.remove(&user_key)? {
            Some(_) => {
                self.db.flush()?;
                info!("Deleted key share for user: {}", user_id);
                Ok(true)
            }
            None => {
                debug!("No key found to delete for user: {}", user_id);
                Ok(false)
            }
        }
    }
}
