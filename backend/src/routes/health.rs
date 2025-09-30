use actix_web::{get, HttpResponse, Result, web};
use serde_json::json;
use sqlx::PgPool;

#[get("/health")]
pub async fn health_check(pool: web::Data<PgPool>) -> Result<HttpResponse> {
    // Test database connection
    let db_status = match sqlx::query("SELECT 1 as health").fetch_one(pool.as_ref()).await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    let response = json!({
        "status": if db_status == "healthy" { "ok" } else { "error" },
        "database": db_status,
        "timestamp": chrono::Utc::now(),
        "service": "mpc-solana-wallet-backend"
    });

    if db_status == "healthy" {
        Ok(HttpResponse::Ok().json(response))
    } else {
        Ok(HttpResponse::ServiceUnavailable().json(response))
    }
}