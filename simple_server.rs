use actix_web::{web, App, HttpServer, HttpResponse, Result};
use serde_json::json;

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "solana-wallet-backend",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn signup(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    
    if username.is_empty() || password.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Username and password are required"
        })));
    }
    
    Ok(HttpResponse::Created().json(json!({
        "message": "User created successfully",
        "user_id": "test-user-id",
        "public_key": "test-public-key"
    })))
}

async fn signin(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    
    if username.is_empty() || password.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Username and password are required"
        })));
    }
    
    Ok(HttpResponse::Ok().json(json!({
        "message": "Sign in successful",
        "token": "test-jwt-token",
        "user_id": "test-user-id"
    })))
}

async fn profile() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "user_id": "test-user-id",
        "email": "test@example.com",
        "public_key": "test-public-key"
    })))
}

async fn balance() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "balance": 0,
        "currency": "SOL"
    })))
}

async fn quote(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "quote_id": "test-quote-id",
        "input_amount": 1000000000,
        "output_amount": 500000000,
        "price_impact": 0.01
    })))
}

async fn send(req: web::Json<serde_json::Value>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "signature": "test-signature",
        "status": "success"
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting simple Solana wallet server on http://127.0.0.1:8080");
    
    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health))
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/user")
                            .route("/signup", web::post().to(signup))
                            .route("/signin", web::post().to(signin))
                            .route("/profile", web::get().to(profile))
                    )
                    .service(
                        web::scope("/solana")
                            .route("/balance", web::get().to(balance))
                            .route("/quote", web::post().to(quote))
                            .route("/send", web::post().to(send))
                    )
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
