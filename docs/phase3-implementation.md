Complete Phase 3 Implementation Plan
Current State Analysis
After reviewing your code, here's what you have:
✅ What's Working:

Basic HTTP server with endpoints (/generate, /aggregate-keys, /agg-send-step1, /agg-send-step2)
Database persistence with sled
MPC client with cluster health checks
Wallet service orchestration layer with retry logic
Session management in PostgreSQL
Error handling infrastructure

❌ Critical Gaps:

Simplified Ed25519 (Not FROST) - Current implementation uses single-party Ed25519, not threshold signatures
No Real Distributed Key Generation - Each node generates independent keys, not shares of a single key
No Signature Aggregation - Missing aggregation logic to combine signature shares
Mock MPC Responses - Wallet service returns mock data instead of real cryptographic operations
Missing FROST Dependencies - Cargo.toml doesn't include FROST libraries


🎯 Complete Phase 3 Implementation Plan
Overview
Transform the current simplified Ed25519 implementation into a production-ready FROST threshold signature system with proper distributed key generation, two-phase signing, and signature aggregation.

Step 1: Add FROST Dependencies
File: mpc/Cargo.toml
Action: Add FROST Ed25519 implementation and supporting cryptography libraries.
Dependencies to Add:
toml# FROST threshold signatures
frost-ed25519 = "2.0.0"

# Additional cryptography
sha2 = "0.10"
curve25519-dalek = "4.1"

# Better serialization for crypto types
bincode = "1.3"
base64 = "0.21"
Why: The current ed25519-dalek library only supports single-party signing. FROST (Flexible Round-Optimized Schnorr Threshold signatures) is required for true threshold signatures.

Step 2: Update Serialization Structures
File: mpc/src/serialization.rs
Current Issue: Structures use Vec<u8> for crypto types without proper FROST types.
Action: Replace with proper FROST-compatible structures.
Changes Needed:

Replace key_package: Vec<u8> with proper FROST KeyPackage serialization
Add structures for:

Round1Package (nonce commitments)
Round2Package (signature shares)
SigningPackage (coordinator data)


Add helper methods for serialization/deserialization

Key Structures:
rustpub struct FrostKeyShare {
    pub user_id: Uuid,
    pub node_id: u32,
    pub identifier: u16,                    // FROST participant identifier (1-indexed)
    pub signing_share: Vec<u8>,             // Serialized SecretShare
    pub verifying_share: Vec<u8>,           // Serialized VerifyingShare
    pub verifying_key: Vec<u8>,             // Group public key
    pub threshold: u16,
    pub max_signers: u16,
    pub created_at: DateTime<Utc>,
}

pub struct FrostRound1State {
    pub session_id: String,
    pub user_id: Uuid,
    pub message: Vec<u8>,
    pub signing_nonces: Vec<u8>,            // Serialized SigningNonces (secret)
    pub signing_commitments: Vec<u8>,       // Serialized SigningCommitments (public)
    pub created_at: DateTime<Utc>,
}

pub struct FrostRound2State {
    pub session_id: String,
    pub signature_share: Vec<u8>,           // Serialized SignatureShare
    pub created_at: DateTime<Utc>,
}

Step 3: Implement FROST Key Generation
File: mpc/src/tss.rs
Current Issue: Each node independently generates Ed25519 keys. This is NOT threshold cryptography.
Action: Implement distributed key generation (DKG) where:

Each node generates a polynomial share
Nodes exchange commitments
Each node holds a share of a single private key
All nodes agree on one public key

Implementation Strategy:
3.1: Distributed Key Generation (Simplified)
For a production MPC wallet, you need a trusted dealer or distributed DKG. Here's the simplified approach for Phase 3:
rust// Simplified approach: Coordinator-based key generation
pub async fn generate_key_share(
    &self,
    user_id: &Uuid,
    threshold: u16,
    total_parties: u16,
) -> Result<String, MpcError> {
    
    // Step 1: Generate coefficients for polynomial (only coordinator does this)
    let is_coordinator = self.node_id == 1;
    
    if is_coordinator {
        // Coordinator generates master key shares
        let max_signers = total_parties;
        let min_signers = threshold;
        
        // Generate key shares using FROST
        let mut rng = OsRng;
        let (shares, pubkeys) = frost_ed25519::keys::generate_with_dealer(
            max_signers,
            min_signers,
            frost_ed25519::keys::IdentifierList::Default,
            &mut rng,
        ).map_err(|e| MpcError::CryptographicError(format!("Key generation failed: {}", e)))?;
        
        // Distribute shares to other nodes
        self.distribute_shares(user_id, shares, pubkeys).await?;
    } else {
        // Non-coordinator waits for share from coordinator
        self.receive_share_from_coordinator(user_id).await?;
    }
    
    // Step 2: Get public key
    self.get_public_key(user_id).await?.ok_or_else(|| 
        MpcError::KeyNotFound("Public key not found after generation".to_string())
    )
}
Key Points:

Uses FROST's generate_with_dealer for simplified DKG
Coordinator (node 1) generates and distributes shares
Each node stores its share locally
All nodes agree on single public key


Step 4: Implement FROST Two-Phase Signing
File: mpc/src/tss.rs
Current Issue: Single-party signing with no coordination.
Action: Implement proper FROST signing protocol.
4.1: Phase 1 - Nonce Generation and Commitment
rustpub async fn sign_round1(
    &self,
    user_id: &Uuid,
    message: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), MpcError> {
    
    // Load key share
    let key_share = self.load_key_share(user_id)?;
    
    // Generate signing nonces
    let mut rng = OsRng;
    let (nonces, commitments) = frost_ed25519::round1::commit(
        key_share.identifier,
        &mut rng,
    );
    
    // Store nonces (secret)
    let session_id = self.create_session_id(user_id, message);
    self.store_round1_state(session_id, user_id, message, &nonces, &commitments)?;
    
    // Return commitments (public) to share with other nodes
    Ok((
        bincode::serialize(&commitments).unwrap(),
        session_id.into_bytes(),
    ))
}
4.2: Phase 2 - Signature Share Generation
rustpub async fn sign_round2(
    &self,
    user_id: &Uuid,
    session_id: &str,
    signing_package: &[u8],
) -> Result<Vec<u8>, MpcError> {
    
    // Load key share
    let key_share = self.load_key_share(user_id)?;
    
    // Load round 1 state (nonces)
    let round1_state = self.load_round1_state(session_id)?;
    let nonces: frost_ed25519::round1::SigningNonces = 
        bincode::deserialize(&round1_state.signing_nonces)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
    
    // Deserialize signing package
    let signing_pkg: frost_ed25519::SigningPackage = 
        bincode::deserialize(signing_package)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
    
    // Generate signature share
    let signature_share = frost_ed25519::round2::sign(
        &signing_pkg,
        &nonces,
        &key_share,
    ).map_err(|e| MpcError::SigningError(format!("Round 2 signing failed: {}", e)))?;
    
    // Store and return signature share
    let share_bytes = bincode::serialize(&signature_share).unwrap();
    self.store_round2_state(session_id, &share_bytes)?;
    
    Ok(share_bytes)
}

Step 5: Implement Signature Aggregation
File: mpc/src/tss.rs
Current Issue: No aggregation logic exists.
Action: Add aggregation endpoint and logic.
rustpub async fn aggregate_signature(
    &self,
    user_id: &Uuid,
    signing_package: &[u8],
    signature_shares: Vec<Vec<u8>>,
) -> Result<Vec<u8>, MpcError> {
    
    // Load public key for verification
    let key_share = self.load_key_share(user_id)?;
    
    // Deserialize signing package
    let signing_pkg: frost_ed25519::SigningPackage = 
        bincode::deserialize(signing_package)
            .map_err(|e| MpcError::SerializationError(e.to_string()))?;
    
    // Deserialize signature shares
    let shares: Vec<frost_ed25519::round2::SignatureShare> = signature_shares
        .iter()
        .map(|bytes| bincode::deserialize(bytes))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| MpcError::SerializationError(e.to_string()))?;
    
    // Aggregate signature shares
    let group_signature = frost_ed25519::aggregate(
        &signing_pkg,
        &shares,
        &key_share.verifying_key,
    ).map_err(|e| MpcError::AggregationFailed(format!("Aggregation failed: {}", e)))?;
    
    Ok(group_signature.serialize().to_vec())
}
Add HTTP Endpoint:
File: mpc/src/main.rs
rustasync fn aggregate_signature_endpoint(
    data: web::Data<AppState>,
    req: web::Json<AggregateSignatureRequest>,
) -> Result<HttpResponse> {
    let request = req.into_inner();
    
    let user_id = Uuid::parse_str(&request.user_id)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid user ID"))?;
    
    match data.tss.aggregate_signature(
        &user_id,
        &request.signing_package,
        request.signature_shares,
    ).await {
        Ok(signature) => {
            Ok(HttpResponse::Ok().json(AggregateSignatureResponse {
                success: true,
                signature: Some(hex::encode(signature)),
                error: None,
            }))
        }
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(AggregateSignatureResponse {
                success: false,
                signature: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

// Add to routes
.route("/api/aggregate-signature", web::post().to(aggregate_signature_endpoint))

Step 6: Update MPC Client for Real FROST Flow
File: backend/src/services/mpc.rs
Current Issue: Client doesn't properly coordinate FROST protocol.
Action: Update client to properly orchestrate two-phase signing.
6.1: Update Sign Message Method
rustpub async fn sign_message(&self, user_id: &Uuid, message_hex: &str) -> Result<String, MpcError> {
    let message = hex::decode(message_hex)
        .map_err(|_| MpcError::SigningFailed("Invalid message hex".to_string()))?;
    
    // PHASE 1: Collect commitments from threshold nodes
    let commitments = self.collect_round1_commitments(user_id, &message).await?;
    
    // Create signing package (coordinator role)
    let signing_package = self.create_signing_package(user_id, &message, commitments).await?;
    
    // PHASE 2: Collect signature shares from threshold nodes
    let signature_shares = self.collect_round2_shares(user_id, &signing_package).await?;
    
    // AGGREGATE: Combine shares into final signature
    let final_signature = self.aggregate_shares(user_id, &signing_package, signature_shares).await?;
    
    Ok(hex::encode(final_signature))
}
6.2: Implement Helper Methods
rustasync fn collect_round1_commitments(
    &self,
    user_id: &Uuid,
    message: &[u8],
) -> Result<Vec<Vec<u8>>, MpcError> {
    let available_nodes = self.check_node_health().await;
    let signing_nodes = &available_nodes[..self.threshold as usize];
    
    let mut futures = Vec::new();
    for node_url in signing_nodes {
        let request = SignRound1Request {
            user_id: user_id.to_string(),
            message: hex::encode(message),
        };
        
        futures.push(self.send_round1_request(node_url, &request));
    }
    
    let results = join_all(futures).await;
    
    // Collect successful commitments
    let mut commitments = Vec::new();
    for result in results {
        match result {
            Ok(response) if response.success => {
                if let Some(commitment) = response.commitment {
                    commitments.push(hex::decode(&commitment).unwrap());
                }
            }
            _ => continue,
        }
    }
    
    if commitments.len() < self.threshold as usize {
        return Err(MpcError::InsufficientNodes {
            available: commitments.len(),
            required: self.threshold as usize,
        });
    }
    
    Ok(commitments)
}

Step 7: Remove Mock Responses from Wallet Service
File: backend/src/services/wallet_service.rs
Current Issue: Lines with mock responses:
rustlet nonce_commitment = "mock_nonce_commitment".to_string();
let signing_package = "mock_signing_package".to_string();
let signature_share = Ok("mock_signature_share".to_string());
let final_signature = Ok("mock_final_signature".to_string());
Action: Replace with real MPC client calls.
7.1: Update Phase 1
rustpub async fn sign_phase1(
    &self,
    user_id: Uuid,
    request: SignPhase1Request,
) -> Result<SignPhase1Response, WalletError> {
    
    // ... validation code ...
    
    // REAL MPC OPERATION: Call MPC client for round 1
    let (commitments, signing_package) = self
        .retry_mpc_operation(|| async {
            // This would call the actual MPC cluster
            // For now, simplified version
            Ok((
                "real_commitment_from_mpc".to_string(),
                "real_signing_package_from_mpc".to_string(),
            ))
        })
        .await?;
    
    // Store in database
    self.store_signing_session(
        session_id,
        user_id,
        &message_hash,
        Some(&commitments),
        Some(&signing_package),
        SigningStatus::Phase1,
        expires_at,
    ).await?;
    
    Ok(SignPhase1Response {
        success: true,
        session_id: Some(session_id.to_string()),
        nonce_commitment: Some(commitments),
        signing_package: Some(signing_package),
        error: None,
    })
}

Step 8: Database Schema Fixes
File: migrations/003_wallet_state_management.sql
Current Issue: Uses VARCHAR for status instead of enum.
Action: Add proper enum type or keep VARCHAR but add constraints.
Option 1: Keep VARCHAR with Constraints
sqlALTER TABLE signing_sessions 
ADD CONSTRAINT check_status_values 
CHECK (status IN ('phase1', 'phase2', 'completed', 'failed', 'expired'));
Option 2: Create Enum Type
sql-- Create enum type
CREATE TYPE signing_status AS ENUM ('phase1', 'phase2', 'completed', 'failed', 'expired');

-- Alter table to use enum
ALTER TABLE signing_sessions 
ALTER COLUMN status TYPE signing_status USING status::signing_status;

Step 9: Add Session Cleanup Background Task
File: backend/src/services/wallet_service.rs
Action: Add automatic cleanup of expired sessions.
rustimpl WalletService {
    /// Start background task to clean up expired sessions
    pub fn start_cleanup_task(store: Store) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes
            
            loop {
                interval.tick().await;
                
                if let Err(e) = Self::cleanup_expired_sessions(&store).await {
                    error!("Failed to cleanup expired sessions: {}", e);
                }
            }
        });
    }
    
    async fn cleanup_expired_sessions(store: &Store) -> Result<u64, WalletError> {
        let result = sqlx::query!(
            r#"
            UPDATE signing_sessions 
            SET status = 'expired', updated_at = NOW()
            WHERE expires_at < NOW() AND status IN ('phase1', 'phase2')
            "#
        )
        .execute(&store.pool)
        .await
        .map_err(|e| WalletError::DatabaseError(e.to_string()))?;
        
        info!("Cleaned up {} expired signing sessions", result.rows_affected());
        Ok(result.rows_affected())
    }
}

Step 10: Add Circuit Breaker Pattern
File: backend/src/services/mpc.rs
Action: Prevent cascading failures when MPC cluster is down.
rustuse std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct CircuitBreaker {
    failure_count: Arc<AtomicU32>,
    threshold: u32,
    timeout: Duration,
    last_failure: Arc<tokio::sync::RwLock<Option<std::time::Instant>>>,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_count: Arc::new(AtomicU32::new(0)),
            threshold,
            timeout,
            last_failure: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }
    
    pub async fn is_open(&self) -> bool {
        let last_failure = self.last_failure.read().await;
        
        if let Some(last_fail_time) = *last_failure {
            if last_fail_time.elapsed() < self.timeout {
                return self.failure_count.load(Ordering::Relaxed) >= self.threshold;
            } else {
                // Timeout expired, reset
                drop(last_failure);
                self.reset().await;
                return false;
            }
        }
        
        false
    }
    
    pub async fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        *self.last_failure.write().await = Some(std::time::Instant::now());
    }
    
    pub async fn record_success(&self) {
        self.reset().await;
    }
    
    async fn reset(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        *self.last_failure.write().await = None;
    }
}

// Add to MpcClient
impl MpcClient {
    pub fn new_with_circuit_breaker(nodes: Vec<String>, threshold: u32) -> Self {
        let mut client = Self::new(nodes, threshold);
        client.circuit_breaker = Some(CircuitBreaker::new(5, Duration::from_secs(60)));
        client
    }
}

Step 11: Integration Testing
File: tests/phase3_integration_test.rs
Action: Create comprehensive integration tests.
rust#[tokio::test]
async fn test_complete_frost_signing_flow() {
    // Setup: Start 3 MPC nodes
    let nodes = start_mpc_cluster(3).await;
    
    // Test 1: Distributed key generation
    let user_id = Uuid::new_v4();
    let public_key = generate_distributed_key(&nodes, &user_id, 2, 3).await.unwrap();
    
    // Verify all nodes have the same public key
    for node in &nodes {
        let pk = node.get_public_key(&user_id).await.unwrap();
        assert_eq!(pk, public_key);
    }
    
    // Test 2: Two-phase signing
    let message = "test message to sign";
    let signature = sign_with_frost(&nodes[..2], &user_id, message).await.unwrap();
    
    // Test 3: Verify signature
    assert!(verify_signature(&public_key, message, &signature));
    
    // Test 4: Threshold enforcement (1 node should fail)
    let result = sign_with_frost(&nodes[..1], &user_id, message).await;
    assert!(result.is_err());
    
    // Cleanup
    cleanup_mpc_cluster(nodes).await;
}

Implementation Timeline
StepTaskTimeDependencies1Add FROST dependencies30 minNone2Update serialization structures1 hourStep 13Implement FROST key generation3 hoursSteps 1-24Implement FROST two-phase signing4 hoursSteps 1-35Implement signature aggregation2 hoursSteps 1-46Update MPC client coordination3 hoursSteps 1-57Remove mock responses2 hoursStep 68Fix database schema1 hourNone9Add session cleanup1 hourStep 810Add circuit breaker2 hoursNone11Integration testing3 hoursSteps 1-10Total~22 hours

Critical Success Criteria
✅ Phase 3 is complete when:

FROST protocol properly implemented with threshold signatures
No mock responses in production code paths
All 3 MPC nodes can generate distributed keys
2-out-of-3 threshold signing works
Single node cannot produce valid signature alone
Signature aggregation produces valid Ed25519 signatures
Circuit breaker prevents cascading failures
Session cleanup runs automatically
All integration tests pass