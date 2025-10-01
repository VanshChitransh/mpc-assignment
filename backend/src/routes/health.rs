use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use crate::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    database: String,
    service: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub async fn health(data: web::Data<AppState>) -> impl Responder {
    // Check database connection
    let db_status = match sqlx::query("SELECT 1").execute(&data.db).await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        database: db_status.to_string(),
        service: "mpc-solana-wallet-backend".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

// Add config function for route registration
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health));
}