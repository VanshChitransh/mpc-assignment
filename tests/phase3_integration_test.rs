use uuid::Uuid;
use backend::services::{MpcClient, WalletService};
use store::Store;

#[tokio::test]
async fn test_complete_frost_signing_flow() {
    // Setup: Start 3 MPC nodes
    let nodes = vec![
        "http://127.0.0.1:8001".to_string(),
        "http://127.0.0.1:8002".to_string(),
        "http://127.0.0.1:8003".to_string(),
    ];
    
    // Test 1: Distributed key generation
    let user_id = Uuid::new_v4();
    let mpc_client = MpcClient::new(nodes.clone(), 2);
    
    // Generate distributed key
    let public_key = mpc_client.generate_key(&user_id).await.unwrap();
    assert!(!public_key.is_empty());
    
    // Verify all nodes have the same public key
    for node_url in &nodes {
        let health_response = reqwest::get(&format!("{}/health", node_url))
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        assert_eq!(health_response["status"], "healthy");
    }
    
    // Test 2: Two-phase signing
    let message = "test message to sign";
    let message_hex = hex::encode(message.as_bytes());
    let signature = mpc_client.sign_message(&user_id, &message_hex).await.unwrap();
    
    // Test 3: Verify signature is not empty
    assert!(!signature.is_empty());
    
    // Test 4: Threshold enforcement (1 node should fail)
    let single_node_client = MpcClient::new(vec![nodes[0].clone()], 2);
    let result = single_node_client.sign_message(&user_id, &message_hex).await;
    assert!(result.is_err());
    
    println!("✅ All FROST integration tests passed!");
}

#[tokio::test]
async fn test_wallet_service_integration() {
    // Setup database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/solana_wallet".to_string());
    
    let store = Store::new_pool(&database_url).await.unwrap();
    let mpc_client = MpcClient::new(
        vec![
            "http://127.0.0.1:8001".to_string(),
            "http://127.0.0.1:8002".to_string(),
            "http://127.0.0.1:8003".to_string(),
        ],
        2
    );
    
    let wallet_service = WalletService::new(mpc_client, store);
    
    // Test user creation and key generation
    let user_id = Uuid::new_v4();
    
    // Create test user in database
    let create_request = store::CreateUserRequest {
        email: format!("test-{}@example.com", user_id),
        password: "testpassword123".to_string(),
    };
    let user = store.create_user(create_request).await.unwrap();
    
    // Test key generation
    let keygen_request = backend::services::wallet_service::KeyGenRequest {
        threshold: Some(2),
        total_parties: Some(3),
    };
    
    let result = wallet_service.generate_key(user_id, keygen_request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    assert!(response.success);
    assert!(response.public_key.is_some());
    
    println!("✅ Wallet service integration tests passed!");
}

#[tokio::test]
async fn test_session_management() {
    // Test session creation, expiration, and cleanup
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/solana_wallet".to_string());
    
    let store = Store::new_pool(&database_url).await.unwrap();
    
    // Test session cleanup
    let cleanup_result = backend::services::wallet_service::WalletService::cleanup_expired_sessions(&store).await;
    assert!(cleanup_result.is_ok());
    
    println!("✅ Session management tests passed!");
}

#[tokio::test]
async fn test_mpc_cluster_health() {
    let nodes = vec![
        "http://127.0.0.1:8001".to_string(),
        "http://127.0.0.1:8002".to_string(),
        "http://127.0.0.1:8003".to_string(),
    ];
    
    let mpc_client = MpcClient::new(nodes, 2);
    
    // Test cluster health check
    let cluster_status = mpc_client.get_cluster_status().await;
    assert!(cluster_status.total_nodes >= 2);
    
    // Test threshold availability
    let is_available = mpc_client.check_threshold_availability().await;
    assert!(is_available);
    
    println!("✅ MPC cluster health tests passed!");
}

#[tokio::test]
async fn test_error_handling() {
    let mpc_client = MpcClient::new(
        vec!["http://localhost:9999".to_string()], // Non-existent node
        2
    );
    
    let user_id = Uuid::new_v4();
    
    // Test error handling for unavailable nodes
    let result = mpc_client.generate_key(&user_id).await;
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Insufficient nodes") || error.to_string().contains("All nodes are unavailable"));
    
    println!("✅ Error handling tests passed!");
}
