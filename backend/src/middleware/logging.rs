use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpMessage, Result,
};
use actix_web::middleware::Next;
use uuid::Uuid;
use tracing::{info, error};
use std::time::Instant;

/// Structured logging middleware for API requests
pub async fn api_logging_middleware(
    req: ServiceRequest,
    next: Next<ServiceRequest>,
) -> Result<ServiceResponse, Error> {
    let start_time = Instant::now();
    let method = req.method().clone();
    let path = req.path().to_string();
    let user_id = req.extensions().get::<Uuid>().copied();
    
    // Log request
    info!(
        user_id = ?user_id,
        method = %method,
        path = %path,
        "API request started"
    );

    // Process request
    let result = next.call(req).await;
    let duration = start_time.elapsed();

    match &result {
        Ok(resp) => {
            let status = resp.status();
            info!(
                user_id = ?user_id,
                method = %method,
                path = %path,
                status = %status,
                duration_ms = duration.as_millis(),
                "API request completed"
            );
        }
        Err(err) => {
            error!(
                user_id = ?user_id,
                method = %method,
                path = %path,
                error = %err,
                duration_ms = duration.as_millis(),
                "API request failed"
            );
        }
    }

    result
}
