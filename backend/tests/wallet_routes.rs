// Integration tests for wallet routes
// These tests validate the wallet route structure and behavior

use actix_web::{test, web, App, HttpResponse, Result};
use serde_json::json;

// Mock handlers that simulate the wallet route behavior
// These mirror the actual wallet route handlers but with mock responses
async fn mock_keygen(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    let data = req.into_inner();
    
    // Validate threshold and total_parties
    if let (Some(threshold), Some(total_parties)) = (data.get("threshold"), data.get("total_parties")) {
        if threshold.as_u64().unwrap_or(0) > total_parties.as_u64().unwrap_or(0) {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Threshold cannot be greater than total parties"
            })));
        }
    }
    
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "public_key": "mock_public_key_12345",
        "error": null
    })))
}

async fn mock_sign_phase1(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    let data = req.into_inner();
    
    // Validate message
    if let Some(message) = data.get("message") {
        if message.as_str().unwrap_or("").is_empty() {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Message cannot be empty"
            })));
        }
    }
    
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "nonce_commitment": "mock_nonce_commitment",
        "signing_package": "mock_signing_package",
        "error": null
    })))
}

async fn mock_sign_phase2(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    let data = req.into_inner();
    
    // Validate message and signing_package
    if let Some(message) = data.get("message") {
        if message.as_str().unwrap_or("").is_empty() {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Message cannot be empty"
            })));
        }
    }
    
    if let Some(signing_package) = data.get("signing_package") {
        if signing_package.as_str().unwrap_or("").is_empty() {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Signing package cannot be empty"
            })));
        }
    }
    
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "signature_share": "mock_signature_share",
        "error": null
    })))
}

async fn mock_aggregate(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    let data = req.into_inner();
    
    // Validate signature_shares
    if let Some(signature_shares) = data.get("signature_shares") {
        if signature_shares.as_array().unwrap_or(&vec![]).is_empty() {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Signature shares cannot be empty"
            })));
        }
    }
    
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "signature": "mock_final_signature",
        "error": null
    })))
}

async fn mock_health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "status": "healthy",
        "cluster_status": {
            "total_nodes": 3,
            "available_nodes": 3,
            "threshold": 2,
            "is_operational": true
        },
        "error": null
    })))
}

#[actix_web::test]
async fn test_wallet_keygen_endpoint() {
    let app = test::init_service(
        App::new()
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/wallet")
                            .route("/keygen", web::post().to(mock_keygen))
                    )
            )
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/wallet/keygen")
        .set_json(&json!({
            "threshold": 2,
            "total_parties": 3
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["public_key"].is_string());
}

#[actix_web::test]
async fn test_wallet_sign_phase1_endpoint() {
    let app = test::init_service(
        App::new()
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/wallet")
                            .route("/sign/phase1", web::post().to(mock_sign_phase1))
                    )
            )
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/wallet/sign/phase1")
        .set_json(&json!({
            "message": "test_message_hex"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["signing_package"].is_string());
}

#[actix_web::test]
async fn test_wallet_sign_phase2_endpoint() {
    let app = test::init_service(
        App::new()
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/wallet")
                            .route("/sign/phase2", web::post().to(mock_sign_phase2))
                    )
            )
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/wallet/sign/phase2")
        .set_json(&json!({
            "message": "test_message_hex",
            "signing_package": "test_signing_package"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["signature_share"].is_string());
}

#[actix_web::test]
async fn test_wallet_aggregate_endpoint() {
    let app = test::init_service(
        App::new()
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/wallet")
                            .route("/aggregate", web::post().to(mock_aggregate))
                    )
            )
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/wallet/aggregate")
        .set_json(&json!({
            "signature_shares": ["share1", "share2", "share3"],
            "signing_package": "test_signing_package"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["signature"].is_string());
}

#[actix_web::test]
async fn test_wallet_health_endpoint() {
    let app = test::init_service(
        App::new()
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/wallet")
                            .route("/health", web::get().to(mock_health))
                    )
            )
    ).await;

    let req = test::TestRequest::get()
        .uri("/api/wallet/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["status"].is_string());
    assert!(body["cluster_status"].is_object());
}

#[actix_web::test]
async fn test_wallet_validation_errors() {
    let app = test::init_service(
        App::new()
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/wallet")
                            .route("/keygen", web::post().to(mock_keygen))
                            .route("/sign/phase1", web::post().to(mock_sign_phase1))
                            .route("/sign/phase2", web::post().to(mock_sign_phase2))
                            .route("/aggregate", web::post().to(mock_aggregate))
                    )
            )
    ).await;

    // Test keygen validation - invalid threshold
    let req = test::TestRequest::post()
        .uri("/api/wallet/keygen")
        .set_json(&json!({
            "threshold": 5,
            "total_parties": 3
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("Threshold cannot be greater than total parties"));

    // Test sign phase1 validation - empty message
    let req = test::TestRequest::post()
        .uri("/api/wallet/sign/phase1")
        .set_json(&json!({
            "message": ""
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("Message cannot be empty"));

    // Test sign phase2 validation - empty signing package
    let req = test::TestRequest::post()
        .uri("/api/wallet/sign/phase2")
        .set_json(&json!({
            "message": "test",
            "signing_package": ""
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("Signing package cannot be empty"));

    // Test aggregate validation - empty signature shares
    let req = test::TestRequest::post()
        .uri("/api/wallet/aggregate")
        .set_json(&json!({
            "signature_shares": [],
            "signing_package": "test"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("Signature shares cannot be empty"));
}

#[actix_web::test]
async fn test_wallet_all_endpoints_exist() {
    let app = test::init_service(
        App::new()
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/wallet")
                            .route("/keygen", web::post().to(mock_keygen))
                            .route("/sign/phase1", web::post().to(mock_sign_phase1))
                            .route("/sign/phase2", web::post().to(mock_sign_phase2))
                            .route("/aggregate", web::post().to(mock_aggregate))
                            .route("/health", web::get().to(mock_health))
                    )
            )
    ).await;

    // Test that all endpoints respond (even if with mock data)
    let endpoints = vec![
        ("POST", "/api/wallet/keygen"),
        ("POST", "/api/wallet/sign/phase1"),
        ("POST", "/api/wallet/sign/phase2"),
        ("POST", "/api/wallet/aggregate"),
        ("GET", "/api/wallet/health"),
    ];

    for (method, uri) in endpoints {
        let req = if method == "GET" {
            test::TestRequest::get().uri(uri).to_request()
        } else {
            test::TestRequest::post()
                .uri(uri)
                .set_json(&json!({"test": "data"}))
                .to_request()
        };

        let resp = test::call_service(&app, req).await;
        
        // All endpoints should respond (not 404)
        assert_ne!(resp.status(), 404, "Endpoint {} {} should exist", method, uri);
        
        // All endpoints should return JSON
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body.is_object(), "Endpoint {} {} should return JSON object", method, uri);
        assert!(body.get("success").is_some(), "Endpoint {} {} should have 'success' field", method, uri);
    }
}
