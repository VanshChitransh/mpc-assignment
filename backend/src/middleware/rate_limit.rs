use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpMessage, Result, web,
};
use actix_web::middleware::Next;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;
use tracing::{warn, info};

/// Rate limiter state
#[derive(Debug)]
struct RateLimitEntry {
    count: u32,
    reset_time: Instant,
}

/// Rate limiter middleware
#[derive(Clone)]
pub struct RateLimitMiddleware {
    limit: u32,
    window: Duration,
    store: Arc<Mutex<HashMap<Uuid, RateLimitEntry>>>,
}

impl RateLimitMiddleware {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn default() -> Self {
        Self::new(100, Duration::from_secs(60)) // 100 requests per minute
    }
}

impl RateLimitMiddleware {
    pub async fn check_rate_limit(&self, user_id: Uuid) -> Result<(), Error> {
        let mut store = self.store.lock().unwrap();
        let now = Instant::now();

        // Clean up expired entries
        store.retain(|_, entry| entry.reset_time > now);

        let entry = store.entry(user_id).or_insert(RateLimitEntry {
            count: 0,
            reset_time: now + self.window,
        });

        if entry.count >= self.limit {
            warn!("Rate limit exceeded for user {}", user_id);
            return Err(actix_web::error::ErrorTooManyRequests("Rate limit exceeded"));
        }

        entry.count += 1;
        info!("Rate limit check passed for user {}: {}/{}", user_id, entry.count, self.limit);
        Ok(())
    }
}

/// Rate limiting middleware function
pub async fn rate_limit_middleware(
    req: ServiceRequest,
    next: Next<ServiceRequest>,
) -> Result<ServiceResponse, Error> {
    // Extract user ID from request extensions
    if let Some(user_id) = req.extensions().get::<Uuid>() {
        // Get rate limiter from app data
        if let Some(rate_limiter) = req.app_data::<web::Data<RateLimitMiddleware>>() {
            rate_limiter.check_rate_limit(*user_id).await?;
        }
    }

    next.call(req).await
}
