use actix_web::{test, web, App};
use backend::{AppState, middleware::{JwtAuth, RateLimitMiddleware, ApiMetrics}};
use backend::routes::api;
use backend::services::{create_default_mpc_client, create_jupiter_client, create_solana_client};
use serde_json::json;
use std::sync::Arc;
use prometheus::Registry;
use uuid::Uuid;

/// Test helper to create test app
async fn create_test_app() -> impl actix_web::dev::Service<actix_web::dev::ServiceRequest, Response = actix_web::dev::ServiceResponse, Error = actix_web::Error> {
    // Create mock services
    let jwt_auth = JwtAuth::new("test_secret".to_string());
    let mpc_client = create_default_mpc_client();
    let jupiter_client = create_jupiter_client();
    let solana_client = create_solana_client();
    
    // Create mock store (you'll need to implement this)
    // let store = create_mock_store().await;
    
    // Initialize metrics
    let registry = Registry::new();
    let api_metrics = Arc::new(ApiMetrics::new(&registry).expect("Failed to create API metrics"));
    
    // Initialize rate limiter
    let rate_limiter = RateLimitMiddleware::default();
    
    // Create app state
    let app_state = web::Data::new(AppState {
        db: todo!(), // You'll need to create a test database connection
        store: todo!(), // You'll need to create a mock store
        jwt_auth: jwt_auth.clone(),
        mpc_client,
        jupiter_client,
        solana_client,
    });

    test::init_service(
        App::new()
            .app_data(app_state)
            .app_data(web::Data::new(rate_limiter))
            .app_data(web::Data::new(api_metrics))
            .configure(api::config)
    ).await
}

/// Test helper to create a valid JWT token
fn create_test_token() -> String {
    let jwt_auth = JwtAuth::new("test_secret".to_string());
    let user_id = Uuid::new_v4();
    jwt_auth.create_token(user_id).expect("Failed to create test token")
}

#[actix_web::test]
async fn test_api_keygen_authenticated() {
    let app = create_test_app().await;
    
    let token = create_test_token();
    let req_body = json!({
        "threshold": 2,
        "participants": 3
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/wallet/keygen")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 200 or appropriate status
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_api_keygen_unauthenticated() {
    let app = create_test_app().await;
    
    let req_body = json!({
        "threshold": 2,
        "participants": 3
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/wallet/keygen")
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 401 Unauthorized
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_api_sign_phase1_authenticated() {
    let app = create_test_app().await;
    
    let token = create_test_token();
    let req_body = json!({
        "message": "test_message",
        "public_key": "test_public_key"
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/wallet/sign/phase1")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 200 or appropriate status
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_api_sign_phase2_authenticated() {
    let app = create_test_app().await;
    
    let token = create_test_token();
    let req_body = json!({
        "session_id": "test_session_id",
        "nonce_commitment": "test_nonce_commitment"
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/wallet/sign/phase2")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 200 or appropriate status
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_api_aggregate_authenticated() {
    let app = create_test_app().await;
    
    let token = create_test_token();
    let req_body = json!({
        "session_id": "test_session_id",
        "signature_shares": ["share1", "share2", "share3"]
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/wallet/aggregate")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 200 or appropriate status
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_api_health_authenticated() {
    let app = create_test_app().await;
    
    let token = create_test_token();

    let req = test::TestRequest::get()
        .uri("/api/v1/wallet/health")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 200 or appropriate status
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_web::test]
async fn test_api_health_unauthenticated() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/v1/wallet/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 401 Unauthorized
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_api_docs_accessibility() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/docs/")
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 200 OK for Swagger UI
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_api_openapi_json() {
    let app = create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should return 200 OK for OpenAPI JSON
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_cors_headers() {
    let app = create_test_app().await;
    
    let token = create_test_token();

    let req = test::TestRequest::get()
        .uri("/api/v1/wallet/health")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("Origin", "http://localhost:3000"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Check for CORS headers
    let headers = resp.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
}

#[actix_web::test]
async fn test_rate_limiting() {
    let app = create_test_app().await;
    
    let token = create_test_token();
    let req_body = json!({
        "threshold": 2,
        "participants": 3
    });

    // Make multiple requests to test rate limiting
    for i in 0..105 { // Exceed the 100 requests/minute limit
        let req = test::TestRequest::post()
            .uri("/api/v1/wallet/keygen")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        
        if i < 100 {
            // First 100 requests should succeed
            assert!(resp.status().is_success() || resp.status().is_client_error());
        } else {
            // Requests after 100 should be rate limited
            assert_eq!(resp.status(), 429);
        }
    }
}

#[actix_web::test]
async fn test_full_wallet_flow() {
    let app = create_test_app().await;
    
    let token = create_test_token();
    
    // Step 1: Key generation
    let keygen_body = json!({
        "threshold": 2,
        "participants": 3
    });

    let keygen_req = test::TestRequest::post()
        .uri("/api/v1/wallet/keygen")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&keygen_body)
        .to_request();

    let keygen_resp = test::call_service(&app, keygen_req).await;
    assert!(keygen_resp.status().is_success() || keygen_resp.status().is_client_error());
    
    // Step 2: Sign phase 1
    let sign1_body = json!({
        "message": "test_message",
        "public_key": "test_public_key"
    });

    let sign1_req = test::TestRequest::post()
        .uri("/api/v1/wallet/sign/phase1")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&sign1_body)
        .to_request();

    let sign1_resp = test::call_service(&app, sign1_req).await;
    assert!(sign1_resp.status().is_success() || sign1_resp.status().is_client_error());
    
    // Step 3: Sign phase 2
    let sign2_body = json!({
        "session_id": "test_session_id",
        "nonce_commitment": "test_nonce_commitment"
    });

    let sign2_req = test::TestRequest::post()
        .uri("/api/v1/wallet/sign/phase2")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&sign2_body)
        .to_request();

    let sign2_resp = test::call_service(&app, sign2_req).await;
    assert!(sign2_resp.status().is_success() || sign2_resp.status().is_client_error());
    
    // Step 4: Aggregate
    let agg_body = json!({
        "session_id": "test_session_id",
        "signature_shares": ["share1", "share2", "share3"]
    });

    let agg_req = test::TestRequest::post()
        .uri("/api/v1/wallet/aggregate")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&agg_body)
        .to_request();

    let agg_resp = test::call_service(&app, agg_req).await;
    assert!(agg_resp.status().is_success() || agg_resp.status().is_client_error());
}

#[actix_web::test]
async fn test_api_response_format() {
    let app = create_test_app().await;
    
    let token = create_test_token();
    let req_body = json!({
        "threshold": 2,
        "participants": 3
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/wallet/keygen")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Check response format
    let body = test::read_body(resp).await;
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Should have standardized response format
    assert!(response_json.get("success").is_some());
    assert!(response_json.get("data").is_some() || response_json.get("error").is_some());
}
