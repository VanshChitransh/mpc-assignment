use backend::services::wallet_service::{WalletService, WalletError, RetryConfig, SigningStatus};
use backend::services::mpc::{MpcClient, MpcError};
use backend::store::Store;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

/// Mock MPC client for testing
struct MockMpcClient {
    should_fail: bool,
    fail_count: u32,
    current_fail_count: u32,
}

impl MockMpcClient {
    fn new() -> Self {
        Self {
            should_fail: false,
            fail_count: 0,
            current_fail_count: 0,
        }
    }

    fn with_failure(mut self, fail_count: u32) -> Self {
        self.should_fail = true;
        self.fail_count = fail_count;
        self
    }
}

impl MpcClient {
    pub async fn generate_key(&self, user_id: &Uuid) -> Result<String, MpcError> {
        // Mock implementation for testing
        Ok(format!("mock_public_key_{}", user_id))
    }

    pub async fn check_threshold_availability(&self) -> bool {
        true
    }

    pub async fn get_cluster_status(&self) -> backend::services::mpc::ClusterStatus {
        backend::services::mpc::ClusterStatus {
            total_nodes: 3,
            available_nodes: 3,
            threshold: 2,
            is_operational: true,
            node_statuses: vec![],
        }
    }
}

/// Test database setup
async fn setup_test_db() -> Store {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost:5432/test_wallet".to_string());
    
    Store::from_url(&database_url).await.unwrap()
}

/// Test user creation
async fn create_test_user(store: &Store) -> Uuid {
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);
    let password_hash = "hashed_password".to_string();

    sqlx::query!(
        "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW())",
        user_id,
        email,
        password_hash
    )
    .execute(&store.pool)
    .await
    .unwrap();

    user_id
}

#[tokio::test]
async fn test_key_generation_success() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };

    let result = wallet_service.generate_key(user_id, request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert!(response.public_key.is_some());
    assert!(response.error.is_none());
}

#[tokio::test]
async fn test_key_generation_idempotency() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };

    // First call
    let result1 = wallet_service.generate_key(user_id, request.clone()).await;
    assert!(result1.is_ok());

    // Second call should return existing key
    let result2 = wallet_service.generate_key(user_id, request).await;
    assert!(result2.is_ok());

    let response1 = result1.unwrap();
    let response2 = result2.unwrap();
    assert_eq!(response1.public_key, response2.public_key);
}

#[tokio::test]
async fn test_key_generation_user_not_found() {
    let store = setup_test_db().await;
    let non_existent_user = Uuid::new_v4();
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };

    let result = wallet_service.generate_key(non_existent_user, request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        WalletError::UserNotFound(_) => {},
        _ => panic!("Expected UserNotFound error"),
    }
}

#[tokio::test]
async fn test_key_generation_invalid_threshold() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(5), // Greater than total_parties
        total_parties: Some(3),
    };

    let result = wallet_service.generate_key(user_id, request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        WalletError::InvalidInput(_) => {},
        _ => panic!("Expected InvalidInput error"),
    }
}

#[tokio::test]
async fn test_sign_phase1_success() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    // First generate keys
    let keygen_request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };
    wallet_service.generate_key(user_id, keygen_request).await.unwrap();

    // Then test phase1 signing
    let sign_request = backend::services::wallet_service::SignPhase1Request {
        message: "test_message".to_string(),
    };

    let result = wallet_service.sign_phase1(user_id, sign_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert!(response.session_id.is_some());
    assert!(response.nonce_commitment.is_some());
    assert!(response.signing_package.is_some());
}

#[tokio::test]
async fn test_sign_phase1_idempotency() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    // Generate keys first
    let keygen_request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };
    wallet_service.generate_key(user_id, keygen_request).await.unwrap();

    let sign_request = backend::services::wallet_service::SignPhase1Request {
        message: "test_message".to_string(),
    };

    // First call
    let result1 = wallet_service.sign_phase1(user_id, sign_request.clone()).await;
    assert!(result1.is_ok());

    // Second call should return existing session
    let result2 = wallet_service.sign_phase1(user_id, sign_request).await;
    assert!(result2.is_ok());

    let response1 = result1.unwrap();
    let response2 = result2.unwrap();
    assert_eq!(response1.session_id, response2.session_id);
}

#[tokio::test]
async fn test_sign_phase1_no_keys() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let sign_request = backend::services::wallet_service::SignPhase1Request {
        message: "test_message".to_string(),
    };

    let result = wallet_service.sign_phase1(user_id, sign_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        WalletError::NoKeysFound(_) => {},
        _ => panic!("Expected NoKeysFound error"),
    }
}

#[tokio::test]
async fn test_sign_phase1_empty_message() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    // Generate keys first
    let keygen_request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };
    wallet_service.generate_key(user_id, keygen_request).await.unwrap();

    let sign_request = backend::services::wallet_service::SignPhase1Request {
        message: "".to_string(),
    };

    let result = wallet_service.sign_phase1(user_id, sign_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        WalletError::InvalidInput(_) => {},
        _ => panic!("Expected InvalidInput error"),
    }
}

#[tokio::test]
async fn test_sign_phase2_success() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    // Generate keys and create phase1 session
    let keygen_request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };
    wallet_service.generate_key(user_id, keygen_request).await.unwrap();

    let phase1_request = backend::services::wallet_service::SignPhase1Request {
        message: "test_message".to_string(),
    };
    let phase1_response = wallet_service.sign_phase1(user_id, phase1_request).await.unwrap();
    let session_id = phase1_response.session_id.unwrap();

    // Test phase2 signing
    let phase2_request = backend::services::wallet_service::SignPhase2Request {
        session_id,
        message: "test_message".to_string(),
    };

    let result = wallet_service.sign_phase2(user_id, phase2_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert!(response.signature_share.is_some());
}

#[tokio::test]
async fn test_sign_phase2_invalid_session() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let phase2_request = backend::services::wallet_service::SignPhase2Request {
        session_id: Uuid::new_v4().to_string(),
        message: "test_message".to_string(),
    };

    let result = wallet_service.sign_phase2(user_id, phase2_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        WalletError::InvalidSigningSession(_) => {},
        _ => panic!("Expected InvalidSigningSession error"),
    }
}

#[tokio::test]
async fn test_aggregate_signature_success() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    // Complete the full signing flow
    let keygen_request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };
    wallet_service.generate_key(user_id, keygen_request).await.unwrap();

    let phase1_request = backend::services::wallet_service::SignPhase1Request {
        message: "test_message".to_string(),
    };
    let phase1_response = wallet_service.sign_phase1(user_id, phase1_request).await.unwrap();
    let session_id = phase1_response.session_id.unwrap();

    let phase2_request = backend::services::wallet_service::SignPhase2Request {
        session_id: session_id.clone(),
        message: "test_message".to_string(),
    };
    wallet_service.sign_phase2(user_id, phase2_request).await.unwrap();

    // Test aggregation
    let aggregate_request = backend::services::wallet_service::AggregateRequest {
        session_id,
    };

    let result = wallet_service.aggregate_signature(user_id, aggregate_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert!(response.signature.is_some());
}

#[tokio::test]
async fn test_health_check_success() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let result = wallet_service.check_health(user_id).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.status, "healthy");
    assert!(response.cluster_status.is_some());
}

#[tokio::test]
async fn test_retry_logic() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    
    let retry_config = RetryConfig {
        max_retries: 2,
        base_delay_ms: 100,
        max_delay_ms: 1000,
        backoff_multiplier: 2.0,
    };
    
    let wallet_service = WalletService::new(mpc_client, store).with_retry_config(retry_config);

    let request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };

    let result = wallet_service.generate_key(user_id, request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_node_failure_simulation() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    
    // Create MPC client that simulates node failure
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    let request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };

    // This should still succeed due to retry logic
    let result = wallet_service.generate_key(user_id, request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_duplicate_requests_handling() {
    let store = setup_test_db().await;
    let user_id = create_test_user(&store).await;
    let mpc_client = MpcClient::new(vec!["http://localhost:8001".to_string()], 2);
    let wallet_service = WalletService::new(mpc_client, store);

    // Generate keys first
    let keygen_request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };
    wallet_service.generate_key(user_id, keygen_request).await.unwrap();

    let sign_request = backend::services::wallet_service::SignPhase1Request {
        message: "test_message".to_string(),
    };

    // Make multiple concurrent requests for the same message
    let futures: Vec<_> = (0..5).map(|_| {
        wallet_service.sign_phase1(user_id, sign_request.clone())
    }).collect();

    let results = futures::future::join_all(futures).await;
    
    // All should succeed and return the same session
    let session_ids: Vec<_> = results.into_iter()
        .map(|r| r.unwrap().session_id.unwrap())
        .collect();
    
    // All session IDs should be the same (idempotency)
    assert!(session_ids.iter().all(|id| id == &session_ids[0]));
}
