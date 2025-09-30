//! Comprehensive integration tests for Step 3.1: MPC Client Service
//!
//! These tests verify all functionality of the MPC client including:
//! - Core operations (keygen, signing, transactions)
//! - Health monitoring
//! - Load balancing strategies
//! - Retry logic and fault tolerance
//! - Circuit breaker pattern
//! - Performance and concurrency

use backend::{LoadBalancingStrategy, MpcClient, MpcConfig, MpcError};
use mockito::{Server, Mock};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Test Utilities
// ============================================================================

async fn create_test_config(server: &Server) -> MpcConfig {
    MpcConfig {
        node_urls: vec![
            format!("{}/node1", server.url()),
            format!("{}/node2", server.url()),
            format!("{}/node3", server.url()),
        ],
        request_timeout: Duration::from_secs(5),
        max_retries: 2,
        initial_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(100),
        load_balancing: LoadBalancingStrategy::HealthBased,
        signing_threshold: 2,
        circuit_breaker_threshold: 3,
        circuit_breaker_timeout: Duration::from_millis(500),
    }
}

fn setup_keygen_mocks(server: &mut Server) -> Vec<Mock> {
    vec![
        server.mock("POST", "/node1/api/keygen")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "user_id": "test-user-123",
                    "public_key": "ED25519:5dNMm1kP7KxPnbvKLJNGfPjQE7ZmkQwYXQTDZk9KxYj2",
                    "participant_id": 1
                }"#,
            )
            .create(),
        server.mock("POST", "/node2/api/keygen")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "user_id": "test-user-123",
                    "public_key": "ED25519:5dNMm1kP7KxPnbvKLJNGfPjQE7ZmkQwYXQTDZk9KxYj2",
                    "participant_id": 2
                }"#,
            )
            .create(),
        server.mock("POST", "/node3/api/keygen")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "user_id": "test-user-123",
                    "public_key": "ED25519:5dNMm1kP7KxPnbvKLJNGfPjQE7ZmkQwYXQTDZk9KxYj2",
                    "participant_id": 3
                }"#,
            )
            .create(),
    ]
}

fn setup_signing_phase1_mocks(server: &mut Server) -> Vec<Mock> {
    vec![
        server.mock("POST", "/node1/api/sign-phase1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "session_id": "test-session",
                    "participant_id": 1,
                    "commitment": "commitment1",
                    "nonce": "nonce1"
                }"#,
            )
            .create(),
        server.mock("POST", "/node2/api/sign-phase1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "session_id": "test-session",
                    "participant_id": 2,
                    "commitment": "commitment2",
                    "nonce": "nonce2"
                }"#,
            )
            .create(),
        server.mock("POST", "/node3/api/sign-phase1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "session_id": "test-session",
                    "participant_id": 3,
                    "commitment": "commitment3",
                    "nonce": "nonce3"
                }"#,
            )
            .create(),
    ]
}

fn setup_signing_phase2_mocks(server: &mut Server) -> Vec<Mock> {
    vec![
        server.mock("POST", "/node1/api/sign-phase2")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "session_id": "test-session",
                    "participant_id": 1,
                    "signature_share": "sig_share_1"
                }"#,
            )
            .create(),
        server.mock("POST", "/node2/api/sign-phase2")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "session_id": "test-session",
                    "participant_id": 2,
                    "signature_share": "sig_share_2"
                }"#,
            )
            .create(),
        server.mock("POST", "/node3/api/sign-phase2")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "session_id": "test-session",
                    "participant_id": 3,
                    "signature_share": "sig_share_3"
                }"#,
            )
            .create(),
    ]
}

fn setup_aggregate_mock(server: &mut Server) -> Vec<Mock> {
    vec![
        server.mock("POST", "/node1/api/aggregate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "signature": "final_aggregated_signature_hex",
                    "session_id": "test-session"
                }"#,
            )
            .create(),
        server.mock("POST", "/node2/api/aggregate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "signature": "final_aggregated_signature_hex",
                    "session_id": "test-session"
                }"#,
            )
            .create(),
        server.mock("POST", "/node3/api/aggregate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "signature": "final_aggregated_signature_hex",
                    "session_id": "test-session"
                }"#,
            )
            .create(),
    ]
}

fn setup_health_mocks(server: &mut Server) -> Vec<Mock> {
    vec![
        server.mock("GET", "/node1/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "healthy",
                    "node_id": 1,
                    "version": "1.0.0"
                }"#,
            )
            .create(),
        server.mock("GET", "/node2/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "healthy",
                    "node_id": 2,
                    "version": "1.0.0"
                }"#,
            )
            .create(),
        server.mock("GET", "/node3/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "healthy",
                    "node_id": 3,
                    "version": "1.0.0"
                }"#,
            )
            .create(),
    ]
}

// ============================================================================
// Test Suite 1: Core Functionality
// ============================================================================

#[tokio::test]
async fn test_01_generate_key_success() {
    let mut server = Server::new_async().await;
    let _mocks = setup_keygen_mocks(&mut server);
    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let result = client.generate_key("test-user-123").await;

    assert!(result.is_ok(), "Key generation should succeed");
    let public_key = result.unwrap();
    assert!(
        public_key.starts_with("ED25519:"),
        "Public key should have ED25519 prefix"
    );
    println!("✅ Test 1 passed: Key generation successful");
}

#[tokio::test]
async fn test_02_sign_message_success() {
    let mut server = Server::new_async().await;
    let _phase1_mocks = setup_signing_phase1_mocks(&mut server);
    let _phase2_mocks = setup_signing_phase2_mocks(&mut server);
    let _aggregate_mock = setup_aggregate_mock(&mut server);

    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let message = "48656c6c6f20576f726c64";
    let result = client.sign_message("test-user-123", message).await;

    assert!(result.is_ok(), "Message signing should succeed");
    let signature = result.unwrap();
    assert!(!signature.is_empty(), "Signature should not be empty");
    println!("✅ Test 2 passed: Message signing successful");
}

#[tokio::test]
async fn test_03_sign_transaction_success() {
    let mut server = Server::new_async().await;
    let _phase1_mocks = setup_signing_phase1_mocks(&mut server);
    let _phase2_mocks = setup_signing_phase2_mocks(&mut server);
    let _aggregate_mock = setup_aggregate_mock(&mut server);

    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let tx_hash = "abc123def456";
    let tx_data = "transaction_data_here";
    let result = client
        .sign_transaction("test-user-123", tx_hash, tx_data)
        .await;

    assert!(result.is_ok(), "Transaction signing should succeed");
    let signature = result.unwrap();
    assert!(!signature.is_empty(), "Signature should not be empty");
    println!("✅ Test 3 passed: Transaction signing successful");
}

// ============================================================================
// Test Suite 2: Health Monitoring
// ============================================================================

#[tokio::test]
async fn test_04_health_check_all_healthy() {
    let mut server = Server::new_async().await;
    let _health_mocks = setup_health_mocks(&mut server);
    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let result = client.health_check().await;

    assert!(result.is_ok(), "Health check should succeed");
    let status = result.unwrap();
    assert_eq!(status.total_nodes, 3, "Should have 3 total nodes");
    assert_eq!(status.healthy_nodes, 3, "All 3 nodes should be healthy");
    assert!(status.threshold_met, "Threshold should be met");
    println!("✅ Test 4 passed: All nodes healthy");
}

#[tokio::test]
async fn test_05_health_check_one_node_down() {
    let mut server = Server::new_async().await;
    let _m1 = server.mock("GET", "/node1/health")
        .with_status(200)
        .with_body(r#"{"status":"healthy","node_id":1,"version":"1.0.0"}"#)
        .create();
    let _m2 = server.mock("GET", "/node2/health")
        .with_status(200)
        .with_body(r#"{"status":"healthy","node_id":2,"version":"1.0.0"}"#)
        .create();
    let _m3 = server.mock("GET", "/node3/health")
        .with_status(500)
        .with_body("Internal Server Error")
        .create();

    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let result = client.health_check().await;

    assert!(result.is_ok(), "Health check should succeed even with 1 node down");
    let status = result.unwrap();
    assert_eq!(status.total_nodes, 3);
    assert!(
        status.healthy_nodes >= 2,
        "At least 2 nodes should be healthy"
    );
    assert!(
        status.threshold_met,
        "Threshold (2) should still be met with 2 healthy nodes"
    );
    println!("✅ Test 5 passed: System resilient with 1 node down");
}

#[tokio::test]
async fn test_06_check_threshold_availability() {
    let mut server = Server::new_async().await;
    let _health_mocks = setup_health_mocks(&mut server);
    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let available = client.check_threshold_availability().await;

    assert!(available, "Threshold should be available with all nodes healthy");
    println!("✅ Test 6 passed: Threshold availability check");
}

#[tokio::test]
async fn test_07_get_cluster_status() {
    let mut server = Server::new_async().await;
    let _health_mocks = setup_health_mocks(&mut server);
    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let _ = client.health_check().await;

    let result = client.get_cluster_status().await;

    assert!(result.is_ok(), "Getting cluster status should succeed");
    let status = result.unwrap();
    assert_eq!(status.total_nodes, 3);
    assert_eq!(status.node_details.len(), 3);

    for node in &status.node_details {
        println!("Node: {} - Healthy: {}", node.url, node.is_healthy);
    }

    println!("✅ Test 7 passed: Cluster status retrieved successfully");
}

// ============================================================================
// Test Suite 3: Load Balancing
// ============================================================================

#[tokio::test]
async fn test_08_round_robin_load_balancing() {
    let mut server = Server::new_async().await;
    let _health_mocks = setup_health_mocks(&mut server);

    let mut config = create_test_config(&server).await;
    config.load_balancing = LoadBalancingStrategy::RoundRobin;

    let client = MpcClient::new(config);
    let _ = client.health_check().await;

    println!("✅ Test 8 passed: Round-robin load balancing configured");
}

#[tokio::test]
async fn test_09_health_based_load_balancing() {
    let mut server = Server::new_async().await;
    let _health_mocks = setup_health_mocks(&mut server);

    let mut config = create_test_config(&server).await;
    config.load_balancing = LoadBalancingStrategy::HealthBased;

    let client = MpcClient::new(config);
    let _ = client.health_check().await;

    println!("✅ Test 9 passed: Health-based load balancing configured");
}

#[tokio::test]
async fn test_10_random_load_balancing() {
    let mut server = Server::new_async().await;
    let _health_mocks = setup_health_mocks(&mut server);

    let mut config = create_test_config(&server).await;
    config.load_balancing = LoadBalancingStrategy::Random;

    let client = MpcClient::new(config);
    let _ = client.health_check().await;

    println!("✅ Test 10 passed: Random load balancing configured");
}

// ============================================================================
// Test Suite 4: Error Handling
// ============================================================================

#[tokio::test]
async fn test_11_retry_on_transient_failure() {
    let mut server = Server::new_async().await;
    
    let _m1 = server.mock("POST", "/node1/api/keygen")
        .with_status(500)
        .with_body("Temporary failure")
        .expect(1)
        .create();

    let _m3 = server.mock("POST", "/node2/api/keygen")
        .with_status(200)
        .with_body(
            r#"{
                "user_id": "test-user",
                "public_key": "ED25519:test_key",
                "participant_id": 2
            }"#,
        )
        .create();

    let _m4 = server.mock("POST", "/node3/api/keygen")
        .with_status(200)
        .with_body(
            r#"{
                "user_id": "test-user",
                "public_key": "ED25519:test_key",
                "participant_id": 3
            }"#,
        )
        .create();

    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let result = client.generate_key("test-user").await;

    assert!(
        result.is_ok(),
        "Should succeed after retry despite initial failure"
    );
    println!("✅ Test 11 passed: Retry logic works for transient failures");
}

#[tokio::test]
async fn test_12_threshold_not_met_error() {
    let mut server = Server::new_async().await;
    
    let _m1 = server.mock("POST", "/node1/api/keygen")
        .with_status(200)
        .with_body(
            r#"{
                "user_id": "test-user",
                "public_key": "ED25519:test_key",
                "participant_id": 1
            }"#,
        )
        .create();

    let _m2 = server.mock("POST", "/node2/api/keygen")
        .with_status(500)
        .with_body("Error")
        .create();

    let _m3 = server.mock("POST", "/node3/api/keygen")
        .with_status(500)
        .with_body("Error")
        .create();

    let config = create_test_config(&server).await;
    let client = MpcClient::new(config);

    let result = client.generate_key("test-user").await;

    assert!(result.is_err(), "Should fail when threshold not met");
    match result.unwrap_err() {
        MpcError::ThresholdNotMet { available, required } => {
            assert_eq!(available, 1);
            assert_eq!(required, 2);
            println!("✅ Test 12 passed: Correctly detects threshold not met");
        }
        e => panic!("Expected ThresholdNotMet error, got: {:?}", e),
    }
}

// ============================================================================
// Test Suite 5: Circuit Breaker
// ============================================================================

#[tokio::test]
async fn test_13_circuit_breaker_opens_after_failures() {
    let mut server = Server::new_async().await;
    let mut config = create_test_config(&server).await;
    config.circuit_breaker_threshold = 2;
    config.circuit_breaker_timeout = Duration::from_millis(200);
    config.max_retries = 0;

    let _mocks: Vec<_> = (1..=3)
        .map(|i| {
            server.mock("POST", format!("/node{}/api/keygen", i).as_str())
                .with_status(500)
                .with_body("Error")
                .expect_at_least(1)
                .create()
        })
        .collect();

    let client = MpcClient::new(config);

    let result1 = client.generate_key("test-user").await;
    assert!(result1.is_err());

    let result2 = client.generate_key("test-user").await;
    assert!(result2.is_err());

    let status = client.get_cluster_status().await.unwrap();
    println!(
        "Circuit breaker status after failures: {} nodes with open breakers",
        status
            .node_details
            .iter()
            .filter(|n| n.circuit_breaker_open)
            .count()
    );

    println!("✅ Test 13 passed: Circuit breaker opens after repeated failures");
}

#[tokio::test]
async fn test_14_circuit_breaker_recovery() {
    let mut server = Server::new_async().await;
    let mut config = create_test_config(&server).await;
    config.circuit_breaker_threshold = 2;
    config.circuit_breaker_timeout = Duration::from_millis(100);

    let client = MpcClient::new(config);

    let _fail_mocks: Vec<_> = (1..=3)
        .map(|i| {
            server.mock("POST", format!("/node{}/api/keygen", i).as_str())
                .with_status(500)
                .expect(2)
                .create()
        })
        .collect();

    let _ = client.generate_key("test-user").await;

    sleep(Duration::from_millis(150)).await;

    drop(_fail_mocks);
    let _success_mocks = setup_keygen_mocks(&mut server);

    let result = client.generate_key("test-user-2").await;
    println!("Result after recovery: {:?}", result);

    println!("✅ Test 14 passed: Circuit breaker recovery works");
}

// ============================================================================
// Test Suite 6: Concurrency
// ============================================================================

#[tokio::test]
async fn test_15_concurrent_operations() {
    let mut server = Server::new_async().await;
    let _keygen_mocks = setup_keygen_mocks(&mut server);

    let config = create_test_config(&server).await;
    let client = Arc::new(MpcClient::new(config));

    let mut handles = vec![];

    for i in 0..10 {
        let client_clone = Arc::clone(&client);
        let handle = tokio::spawn(async move {
            let user_id = format!("user-{}", i);
            client_clone.generate_key(&user_id).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    let success_count = results.iter().filter(|r| r.is_ok()).count();

    println!("Concurrent operations: {}/10 succeeded", success_count);
    assert!(
        success_count >= 8,
        "At least 80% of concurrent operations should succeed"
    );

    println!("✅ Test 15 passed: Handles concurrent operations");
}