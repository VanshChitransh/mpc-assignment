use actix_web::{test, web, App};
use backend::{routes, AppState};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

async fn setup_test_app(db: PgPool) -> impl actix_web::dev::Service
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let mpc_nodes = vec![
        "http://localhost:8001".to_string(),
        "http://localhost:8002".to_string(),
        "http://localhost:8003".to_string(),
    ];

    let mpc_client = Arc::new(backend::services::mpc::MPCClient::new(mpc_nodes));
    
    let app_state = AppState {
        db,
        mpc_client,
        jwt_secret: "test_secret_key_12345678".to_string(),
    };

    test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .configure(routes::configure)
    )
    .await
}

#[actix_web::test]
async fn test_complete_signup_flow() {
    // Setup database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/solana_wallet_test".to_string());
    
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Clean up test data
    sqlx::query!("DELETE FROM users WHERE email LIKE 'test_%'")
        .execute(&db)
        .await
        .unwrap();

    let app = setup_test_app(db.clone()).await;

    // Test signup
    let signup_payload = json!({
        "email": "test_user_step32@example.com",
        "password": "SecurePass123!"
    });

    let req = test::TestRequest::post()
        .uri("/api/user/signup")
        .set_json(&signup_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    println!("Signup response status: {}", resp.status());
    
    assert!(resp.status().is_success(), "Signup should succeed");

    let body: serde_json::Value = test::read_body_json(resp).await;
    println!("Signup response: {}", serde_json::to_string_pretty(&body).unwrap());

    assert!(body.get("token").is_some(), "Should return JWT token");
    assert!(body.get("user").is_some(), "Should return user profile");
    
    let user = body.get("user").unwrap();
    assert!(user.get("public_key").is_some(), "Should have MPC-generated public key");

    let token = body.get("token").unwrap().as_str().unwrap();

    // Test signin
    let signin_payload = json!({
        "email": "test_user_step32@example.com",
        "password": "SecurePass123!"
    });

    let req = test::TestRequest::post()
        .uri("/api/user/signin")
        .set_json(&signin_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Signin should succeed");

    // Test get profile (authenticated)
    let req = test::TestRequest::get()
        .uri("/api/user/profile")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Get profile should succeed");

    let profile: serde_json::Value = test::read_body_json(resp).await;
    println!("Profile: {}", serde_json::to_string_pretty(&profile).unwrap());

    // Test wallet status
    let req = test::TestRequest::get()
        .uri("/api/user/wallet-status")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Wallet status should succeed");

    let status: serde_json::Value = test::read_body_json(resp).await;
    println!("Wallet status: {}", serde_json::to_string_pretty(&status).unwrap());

    assert!(status.get("mpc_health").is_some(), "Should include MPC health");
}

#[actix_web::test]
async fn test_validation_errors() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/solana_wallet_test".to_string());
    
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let app = setup_test_app(db).await;

    // Test invalid email
    let payload = json!({
        "email": "invalid-email",
        "password": "SecurePass123!"
    });

    let req = test::TestRequest::post()
        .uri("/api/user/signup")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should reject invalid email");

    // Test short password
    let payload = json!({
        "email": "test@example.com",
        "password": "short"
    });

    let req = test::TestRequest::post()
        .uri("/api/user/signup")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should reject short password");
}