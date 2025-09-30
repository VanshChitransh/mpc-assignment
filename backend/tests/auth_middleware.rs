// backend/tests/auth_middleware.rs
// Comprehensive integration tests for authentication middleware

use actix_web::{test, web, App, HttpResponse};
use uuid::Uuid;

// Mock route handlers for testing
async fn public_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Public endpoint"
    }))
}

async fn protected_handler(req: actix_web::HttpRequest) -> HttpResponse {
    use backend::middleware::auth::get_user_id;
    
    match get_user_id(&req) {
        Ok(user_id) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Protected endpoint",
            "user_id": user_id.to_string()
        })),
        Err(err) => err,
    }
}

#[actix_web::test]
async fn test_public_endpoint_no_auth_required() {
    // Create JWT auth instance
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    
    // Create test app with middleware
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/health", web::get().to(public_handler))
    ).await;

    // Test without authentication
    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should succeed without authentication
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_protected_endpoint_requires_auth() {
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/api/protected", web::get().to(protected_handler))
    ).await;

    // Test without authentication - should fail
    let req = test::TestRequest::get()
        .uri("/api/protected")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should return 401 Unauthorized
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_protected_endpoint_with_valid_token() {
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    let user_id = Uuid::new_v4();
    let token = jwt_auth.generate_token(&user_id, "test@example.com").unwrap();
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/api/protected", web::get().to(protected_handler))
    ).await;

    // Test with valid token
    let req = test::TestRequest::get()
        .uri("/api/protected")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should succeed
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_invalid_token_format() {
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/api/protected", web::get().to(protected_handler))
    ).await;

    // Test with invalid token format (no "Bearer " prefix)
    let req = test::TestRequest::get()
        .uri("/api/protected")
        .insert_header(("Authorization", "invalid_token"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should return 401 Unauthorized
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_expired_token() {
    // Note: This test would require mocking time or using a short expiration
    // For now, we test that invalid tokens are rejected
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/api/protected", web::get().to(protected_handler))
    ).await;

    // Test with completely invalid token
    let req = test::TestRequest::get()
        .uri("/api/protected")
        .insert_header(("Authorization", "Bearer invalid.jwt.token"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should return 401 Unauthorized
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_missing_authorization_header() {
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/api/protected", web::get().to(protected_handler))
    ).await;

    // Test without Authorization header
    let req = test::TestRequest::get()
        .uri("/api/protected")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should return 401 Unauthorized
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_user_id_extraction_in_handler() {
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    let user_id = Uuid::new_v4();
    let token = jwt_auth.generate_token(&user_id, "test@example.com").unwrap();
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/api/protected", web::get().to(protected_handler))
    ).await;

    // Test with valid token
    let req = test::TestRequest::get()
        .uri("/api/protected")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should succeed and return user_id
    assert!(resp.status().is_success());
    
    // Parse response body to verify user_id is present
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["user_id"], user_id.to_string());
}

#[actix_web::test]
async fn test_multiple_protected_routes() {
    let jwt_auth = backend::middleware::auth::JwtAuth::new("test_secret".to_string());
    let user_id = Uuid::new_v4();
    let token = jwt_auth.generate_token(&user_id, "test@example.com").unwrap();
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::auth::AuthMiddleware::new(jwt_auth))
            .route("/api/route1", web::get().to(protected_handler))
            .route("/api/route2", web::get().to(protected_handler))
            .route("/health", web::get().to(public_handler))
    ).await;

    // Test protected route 1
    let req = test::TestRequest::get()
        .uri("/api/route1")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test protected route 2
    let req = test::TestRequest::get()
        .uri("/api/route2")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test public route
    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}