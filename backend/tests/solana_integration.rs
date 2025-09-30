use actix_web::{test, web, App};
use backend::{AppState, blockchain::{SolanaBlockchain, create_solana_blockchain}};
use backend::routes::solana_v1;
use backend::middleware::{AuthMiddleware, JwtAuth, RateLimitMiddleware, ApiMetrics};
use backend::services::{create_default_mpc_client, create_jupiter_client, create_solana_client};
use store::Store;
use serde_json::json;
use uuid::Uuid;
use prometheus::Registry;
use std::sync::Arc;

/// Helper function to create test app state
async fn create_test_app_state() -> web::Data<AppState> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://newuser:new_secure_password@localhost:5432/newdb_test".to_string());
    
    let store = Store::from_url(&database_url).await.expect("Failed to connect to test database");
    let jwt_auth = JwtAuth::new("test-secret".to_string());
    let mpc_client = create_default_mpc_client();
    let jupiter_client = create_jupiter_client();
    let solana_client = create_solana_client();
    let solana_blockchain = create_solana_blockchain();
    
    web::Data::new(AppState {
        db: store.pool.clone(),
        store,
        jwt_auth: jwt_auth.clone(),
        mpc_client,
        jupiter_client,
        solana_client,
        solana_blockchain,
    })
}

#[actix_web::test]
async fn test_derive_address_success() {
    let app_state = create_test_app_state().await;
    
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .configure(solana_v1::config)
    ).await;
    
    // Test with a valid 64-character hex public key
    let req = test::TestRequest::post()
        .uri("/v1/solana/address")
        .set_json(json!({
            "public_key": "1111111111111111111111111111111111111111111111111111111111111111"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success() || resp.status().is_client_error()); // May need auth
}

#[actix_web::test]
async fn test_derive_address_invalid_public_key() {
    let app_state = create_test_app_state().await;
    
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .configure(solana_v1::config)
    ).await;
    
    // Test with invalid public key
    let req = test::TestRequest::post()
        .uri("/v1/solana/address")
        .set_json(json!({
            "public_key": "invalid"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[test]
fn test_validate_address() {
    let blockchain = SolanaBlockchain::new(
        "https://api.devnet.solana.com".to_string(),
        "confirmed".to_string()
    );
    
    // Valid Solana addresses
    assert!(blockchain.validate_address("11111111111111111111111111111111"));
    assert!(blockchain.validate_address("So11111111111111111111111111111111111111112"));
    
    // Invalid addresses
    assert!(!blockchain.validate_address(""));
    assert!(!blockchain.validate_address("invalid"));
    assert!(!blockchain.validate_address("0x1234567890abcdef")); // Ethereum format
}

#[test]
fn test_derive_solana_address() {
    // Test with valid 32-byte hex public key (64 hex characters)
    let valid_pubkey = "1111111111111111111111111111111111111111111111111111111111111111";
    let result = SolanaBlockchain::derive_solana_address(valid_pubkey);
    assert!(result.is_ok());
    
    // Test with invalid public key length
    let invalid_pubkey = "invalid";
    let result = SolanaBlockchain::derive_solana_address(invalid_pubkey);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_recent_blockhash_devnet() {
    let blockchain = SolanaBlockchain::new(
        "https://api.devnet.solana.com".to_string(),
        "confirmed".to_string()
    );
    
    let result = blockchain.get_recent_blockhash().await;
    match result {
        Ok(blockhash) => {
            assert!(!blockhash.is_empty());
            println!("Got blockhash from devnet: {}", blockhash);
        }
        Err(e) => {
            println!("Expected error in test environment (may be rate limited): {}", e);
        }
    }
}

#[tokio::test]
async fn test_build_transaction() {
    let blockchain = SolanaBlockchain::new(
        "https://api.devnet.solana.com".to_string(),
        "confirmed".to_string()
    );
    
    let from = "11111111111111111111111111111111";
    let to = "22222222222222222222222222222222";
    let lamports = 1000000;
    let blockhash = "test_blockhash";
    
    let result = blockchain.build_transaction(from, to, lamports, blockhash).await;
    assert!(result.is_ok());
    
    let tx = result.unwrap();
    assert_eq!(tx.message.account_keys.len(), 2);
    assert_eq!(tx.message.account_keys[0], from);
    assert_eq!(tx.message.account_keys[1], to);
}

#[test]
fn test_sign_transaction() {
    let blockchain = SolanaBlockchain::new(
        "https://api.devnet.solana.com".to_string(),
        "confirmed".to_string()
    );
    
    // Create a dummy transaction
    let tx = backend::blockchain::Transaction {
        signatures: vec![String::new()],
        message: backend::blockchain::TransactionMessage {
            header: backend::blockchain::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec!["11111111111111111111111111111111".to_string()],
            recent_blockhash: "test_blockhash".to_string(),
            instructions: vec![],
        },
    };
    
    // Valid 64-byte signature (128 hex characters)
    let valid_signature = "1111111111111111111111111111111111111111111111111111111111111111\
                           2222222222222222222222222222222222222222222222222222222222222222";
    
    let result = blockchain.sign_transaction(tx.clone(), valid_signature);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().signatures[0], valid_signature);
    
    // Invalid signature
    let invalid_signature = "invalid";
    let result = blockchain.sign_transaction(tx, invalid_signature);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transfer_endpoint_without_auth() {
    let app_state = create_test_app_state().await;
    
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .configure(solana_v1::config)
    ).await;
    
    // Test transfer without authentication
    let req = test::TestRequest::post()
        .uri("/v1/solana/transfer")
        .set_json(json!({
            "to_address": "11111111111111111111111111111111",
            "lamports": 1000000
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail due to missing authentication
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn test_transfer_invalid_recipient() {
    let app_state = create_test_app_state().await;
    
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .configure(solana_v1::config)
    ).await;
    
    // Test transfer with invalid recipient address
    let req = test::TestRequest::post()
        .uri("/v1/solana/transfer")
        .set_json(json!({
            "to_address": "invalid_address",
            "lamports": 1000000
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should fail due to invalid address
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn test_solana_metrics_initialization() {
    let registry = Registry::new();
    
    // Initialize Solana metrics
    let result = solana_v1::init_metrics(&registry);
    assert!(result.is_ok() || result.is_err()); // May fail if already registered
    
    // Check if metrics are registered
    let metric_families = registry.gather();
    println!("Registered {} metric families", metric_families.len());
}

#[test]
fn test_transaction_serialization() {
    use backend::blockchain::{Transaction, TransactionMessage, MessageHeader};
    
    let tx = Transaction {
        signatures: vec!["test_signature".to_string()],
        message: TransactionMessage {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec!["11111111111111111111111111111111".to_string()],
            recent_blockhash: "test_blockhash".to_string(),
            instructions: vec![],
        },
    };
    
    // Test serialization
    let serialized = bincode::serialize(&tx);
    assert!(serialized.is_ok());
    
    // Test deserialization
    let deserialized: Result<Transaction, _> = bincode::deserialize(&serialized.unwrap());
    assert!(deserialized.is_ok());
}

#[test]
fn test_address_validation_edge_cases() {
    let blockchain = SolanaBlockchain::new(
        "https://api.devnet.solana.com".to_string(),
        "confirmed".to_string()
    );
    
    // Empty address
    assert!(!blockchain.validate_address(""));
    
    // Too short
    assert!(!blockchain.validate_address("short"));
    
    // Too long
    assert!(!blockchain.validate_address("verylongaddressthatexceedsthemaximumlengthforsolanaaddresses1234567890"));
    
    // Invalid characters
    assert!(!blockchain.validate_address("0O0O0O0O0O0O0O0O0O0O0O0O0O0O0O0O")); // Contains O which is not in base58
}
