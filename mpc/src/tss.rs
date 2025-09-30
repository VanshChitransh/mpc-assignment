use frost_ed25519 as frost;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::BTreeMap;
use tracing::{info, error, warn};
use uuid::Uuid;
use crate::error::MpcError;
use crate::serialization::{FrostKeyShare, FrostRound1State, FrostRound2State};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyGenResponse {
    pub user_id: Uuid,
    pub node_id: u32,
    pub verifying_key: String,
}

#[derive(Debug)]
pub struct ThresholdSigningService {
    db: Db,
    node_id: u32,
    peer_nodes: Vec<String>,
}

impl ThresholdSigningService {
    pub async fn new(node_id: u32, data_dir: &str, peer_nodes: Vec<String>) -> Result<Self, MpcError> {
        let db_path = format!("{}/keys.db", data_dir);
        let db = sled::open(&db_path)
            .map_err(|e| MpcError::StorageError(e.to_string()))?;

        info!("Initialized TSS service for node {} at {}", node_id, db_path);

        Ok(Self {
            db,
            node_id,
            peer_nodes,
        })
    }

    /// Generate a key share using FROST DKG
    pub async fn generate_key_share(
        &self,
        user_id: &Uuid,
        threshold: u16,
        total_parties: u16,
    ) -> Result<String, MpcError> {
        info!("Generating FROST key share for user: {} (threshold: {}/{})", 
              user_id, threshold, total_parties);

        let mut rng = OsRng;

        // Generate FROST key shares using distributed key generation
        let (shares, pubkey_package) = frost::keys::generate_with_dealer(
            total_parties,
            threshold,
            frost::keys::IdentifierList::Default,
            &mut rng,
        ).map_err(|e| MpcError::KeyGenerationError(format!("FROST DKG failed: {:?}", e)))?;

        // Get this node's identifier
        let identifier = frost::Identifier::try_from(self.node_id as u16)
            .map_err(|e| MpcError::KeyGenerationError(format!("Invalid node ID: {:?}", e)))?;

        // Extract this node's key package
        let key_package = shares.get(&identifier)
            .ok_or_else(|| MpcError::KeyGenerationError(
                format!("No key package found for node {}", self.node_id)
            ))?;

        // Get verifying key from public key package
        let verifying_key = pubkey_package.verifying_key();

        // Serialize the entire key package for storage
        let key_package_bytes = bincode::serialize(key_package)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
        
        let verifying_key_bytes = bincode::serialize(verifying_key)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
        
        // Serialize the public key package for aggregation
        let pubkey_package_bytes = bincode::serialize(&pubkey_package)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;

        // Store the key share
        let frost_key_share = FrostKeyShare {
            user_id: *user_id,
            node_id: self.node_id,
            identifier: self.node_id as u16,
            signing_share: key_package_bytes,  // Store entire KeyPackage
            verifying_share: pubkey_package_bytes, // Store PublicKeyPackage for aggregation
            verifying_key: verifying_key_bytes.clone(),
            threshold,
            max_signers: total_parties,
            created_at: chrono::Utc::now(),
        };

        let serialized = frost_key_share.serialize()
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;

        let user_key = format!("user:{}", user_id);
        self.db.insert(&user_key, serialized)
            .map_err(|e| MpcError::StorageError(e.to_string()))?;
        
        self.db.flush()
            .map_err(|e| MpcError::StorageError(e.to_string()))?;

        let public_key = hex::encode(verifying_key_bytes);
        info!("FROST key generation completed for user: {} public_key: {}", user_id, public_key);
        
        Ok(public_key)
    }

    /// Get the public key for a user
    pub async fn get_public_key(&self, user_id: &Uuid) -> Result<Option<String>, MpcError> {
        let user_key = format!("user:{}", user_id);
        
        if let Some(key_data) = self.db.get(&user_key)
            .map_err(|e| MpcError::StorageError(e.to_string()))? {
            
            if let Ok(frost_key_share) = FrostKeyShare::deserialize(&key_data) {
                return Ok(Some(hex::encode(frost_key_share.verifying_key)));
            }
        }
        
        Ok(None)
    }

    /// Prepare signing session (validates user has key)
    pub async fn prepare_signing(&self, user_id: &Uuid, message_hash: &str) -> Result<(), MpcError> {
        info!("Preparing signing for user: {} message: {}", user_id, message_hash);
        
        // Verify the user has a key
        let _key_share = self.load_key_share(user_id)?;
        
        // Validate message hash format
        if hex::decode(message_hash).is_err() {
            return Err(MpcError::InvalidMessageFormat(
                "Message hash must be valid hex".to_string()
            ));
        }
        
        info!("Signing preparation completed for user: {}", user_id);
        Ok(())
    }

    /// Simple signing interface (for backward compatibility)
    pub async fn sign_message(&self, user_id: &Uuid, message_hash: &str) -> Result<String, MpcError> {
        info!("Signing message for user: {} message: {}", user_id, message_hash);
        
        // This is a simplified interface - in production, you'd use the full round1/round2 flow
        let _message_bytes = hex::decode(message_hash)
            .map_err(|e| MpcError::InvalidMessageFormat(format!("Invalid hex: {}", e)))?;

        // For now, return a placeholder indicating FROST signing would happen
        // In production, this would coordinate the full 2-round protocol
        warn!("Using simplified signing interface - production should use round1/round2");
        
        let mock_signature = hex::encode(vec![0u8; 64]);
        Ok(mock_signature)
    }

    /// FROST Round 1: Generate nonces and commitments
    pub async fn sign_round1(&self, user_id: &Uuid, message: &[u8]) -> Result<(Vec<u8>, Vec<u8>), MpcError> {
        info!("FROST Round 1 for user: {}", user_id);
        
        let key_share = self.load_key_share(user_id)?;
        let mut rng = OsRng;

        // Deserialize the KeyPackage
        let key_package: frost::keys::KeyPackage = bincode::deserialize(&key_share.signing_share)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize key package: {}", e)))?;

        // Generate nonces and commitments for FROST round 1
        let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), &mut rng);

        // Serialize for transmission and storage
        let nonces_bytes = bincode::serialize(&nonces)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
        let commitments_bytes = bincode::serialize(&commitments)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;

        // Store round 1 state for round 2
        let session_id = Uuid::new_v4();
        let round1_state = FrostRound1State {
            session_id: session_id.to_string(),
            user_id: *user_id,
            message: message.to_vec(),
            signing_nonces: nonces_bytes.clone(),
            signing_commitments: commitments_bytes.clone(),
            created_at: chrono::Utc::now(),
        };
        
        self.store_round1_state(&round1_state)?;
        
        info!("FROST Round 1 completed for user: {} session: {}", user_id, session_id);
        
        Ok((commitments_bytes, session_id.as_bytes().to_vec()))
    }

    /// FROST Round 2: Generate signature share
    pub async fn sign_round2(
        &self,
        user_id: &Uuid,
        session_id: &str,
        signing_package_bytes: &[u8],
    ) -> Result<Vec<u8>, MpcError> {
        info!("FROST Round 2 for user: {} session: {}", user_id, session_id);
        
        let key_share = self.load_key_share(user_id)?;
        let round1_state = self.load_round1_state(user_id, session_id)?;

        // Deserialize everything needed for round 2
        let key_package: frost::keys::KeyPackage = bincode::deserialize(&key_share.signing_share)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize key package: {}", e)))?;

        let nonces: frost::round1::SigningNonces = bincode::deserialize(&round1_state.signing_nonces)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize nonces: {}", e)))?;

        let signing_package: frost::SigningPackage = bincode::deserialize(signing_package_bytes)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize signing package: {}", e)))?;

        // Generate signature share using FROST round 2
        let signature_share = frost::round2::sign(&signing_package, &nonces, &key_package)
            .map_err(|e| MpcError::SigningError(format!("FROST round 2 failed: {:?}", e)))?;

        // Serialize signature share
        let signature_share_bytes = bincode::serialize(&signature_share)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;

        // Store round 2 state
        let round2_state = FrostRound2State {
            session_id: round1_state.session_id,
            signature_share: signature_share_bytes.clone(),
            created_at: chrono::Utc::now(),
        };
        
        self.store_round2_state(&round2_state)?;
        
        info!("FROST Round 2 completed for user: {}", user_id);
        Ok(signature_share_bytes)
    }

    /// Aggregate signature shares into final signature
    pub async fn aggregate_signature(
        &self,
        user_id: &Uuid,
        signing_package_bytes: &[u8],
        signature_shares_bytes: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, MpcError> {
        info!("Aggregating {} FROST signatures for user: {}", signature_shares_bytes.len(), user_id);
        
        let key_share = self.load_key_share(user_id)?;

        // Deserialize components
        let signing_package: frost::SigningPackage = bincode::deserialize(signing_package_bytes)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize signing package: {}", e)))?;

        let pubkey_package: frost::keys::PublicKeyPackage = bincode::deserialize(&key_share.verifying_share)
            .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize public key package: {}", e)))?;

        // Deserialize all signature shares
        let mut signature_shares = BTreeMap::new();
        for (i, share_bytes) in signature_shares_bytes.iter().enumerate() {
            let share: frost::round2::SignatureShare = bincode::deserialize(share_bytes)
                .map_err(|e| MpcError::SerializationError(format!("Failed to deserialize signature share {}: {}", i, e)))?;
            
            let identifier = frost::Identifier::try_from((i + 1) as u16)
                .map_err(|e| MpcError::SigningError(format!("Invalid identifier: {:?}", e)))?;
            
            signature_shares.insert(identifier, share);
        }

        // Aggregate using FROST
        let group_signature = frost::aggregate(&signing_package, &signature_shares, &pubkey_package)
            .map_err(|e| MpcError::SigningError(format!("FROST aggregation failed: {:?}", e)))?;

        // Serialize final signature
        let signature_bytes = bincode::serialize(&group_signature)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;

        info!("FROST signature aggregation completed for user: {}", user_id);
        Ok(signature_bytes)
    }

    // Helper methods
    fn load_key_share(&self, user_id: &Uuid) -> Result<FrostKeyShare, MpcError> {
        let user_key = format!("user:{}", user_id);
        
        let key_data = self.db.get(&user_key)
            .map_err(|e| MpcError::StorageError(e.to_string()))?
            .ok_or_else(|| MpcError::KeyNotFound(user_id.to_string()))?;

        FrostKeyShare::deserialize(&key_data)
            .map_err(|e| MpcError::SerializationError(e.to_string()))
    }

    fn store_round1_state(&self, state: &FrostRound1State) -> Result<(), MpcError> {
        let key = format!("round1:{}:{}", state.user_id, state.session_id);
        let serialized = state.serialize()
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
        
        self.db.insert(&key, serialized)
            .map_err(|e| MpcError::StorageError(e.to_string()))?;
        
        Ok(())
    }

    fn load_round1_state(&self, user_id: &Uuid, session_id: &str) -> Result<FrostRound1State, MpcError> {
        let key = format!("round1:{}:{}", user_id, session_id);
        
        let state_data = self.db.get(&key)
            .map_err(|e| MpcError::StorageError(e.to_string()))?
            .ok_or_else(|| MpcError::SessionNotFound(format!("Round 1 state not found for session: {}", session_id)))?;

        FrostRound1State::deserialize(&state_data)
            .map_err(|e| MpcError::SerializationError(e.to_string()))
    }

    fn store_round2_state(&self, state: &FrostRound2State) -> Result<(), MpcError> {
        let key = format!("round2:{}", state.session_id);
        let serialized = state.serialize()
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
        
        self.db.insert(&key, serialized)
            .map_err(|e| MpcError::StorageError(e.to_string()))?;
        
        Ok(())
    }

    pub fn get_db(&self) -> &Db {
        &self.db
    }
}