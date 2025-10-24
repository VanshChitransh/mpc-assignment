use actix_web::{web, HttpResponse, Result};
use serde::Serialize;
use chrono::Utc;
use crate::main::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    database: String,
    timestamp: String,
}

#[actix_web::get("/health")]
pub async fn health_check(data: web::Data<AppState>) -> Result<HttpResponse> {
    // CRITICAL FIX: Use data.store.pool instead of data.db
    let db_status = match sqlx::query("SELECT 1")
        .execute(&data.store.pool)
        .await
    {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    Ok(HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        service: "mpc-solana-wallet-backend".to_string(),
        database: db_status.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    }))
}