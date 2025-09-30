// backend/src/middleware/rate_limit.rs
// Fixed version compatible with actix-web 4.x

use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use std::collections::HashMap;
use std::future::{ready, Ready};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use futures_util::future::LocalBoxFuture;
use tracing::{info, warn};
use uuid::Uuid;

/// Rate limiter entry tracking request count and reset time
#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    reset_time: Instant,
}

/// Rate limiter middleware - tracks requests per user
#[derive(Clone)]
pub struct RateLimitMiddleware {
    limit: u32,
    window: Duration,
    store: Arc<Mutex<HashMap<Uuid, RateLimitEntry>>>,
}

impl RateLimitMiddleware {
    /// Create new rate limiter with specified limit and time window
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create default rate limiter (100 requests per minute)
    pub fn default() -> Self {
        Self::new(100, Duration::from_secs(60))
    }

    /// Check if request is within rate limit
    fn check_rate_limit(&self, user_id: Uuid) -> bool {
        let mut store = self.store.lock().unwrap();
        let now = Instant::now();

        // Clean up expired entries
        store.retain(|_, entry| entry.reset_time > now);

        let entry = store.entry(user_id).or_insert(RateLimitEntry {
            count: 0,
            reset_time: now + self.window,
        });

        // Check if limit exceeded
        if entry.count >= self.limit {
            warn!("Rate limit exceeded for user {}: {}/{}", user_id, entry.count, self.limit);
            return false;
        }

        // Increment counter
        entry.count += 1;
        info!("Rate limit check passed for user {}: {}/{}", user_id, entry.count, self.limit);
        true
    }
}

// Implement Transform trait for middleware
impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitService {
            service,
            limiter: self.clone(),
        }))
    }
}

/// Rate limit service wrapper
pub struct RateLimitService<S> {
    service: S,
    limiter: RateLimitMiddleware,
}

impl<S, B> Service<ServiceRequest> for RateLimitService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract user ID from request extensions (set by auth middleware)
        let user_id = req.extensions().get::<Uuid>().copied();

        // If no user ID, allow request (might be public endpoint)
        let should_check = user_id.is_some();
        let rate_limit_passed = if let Some(uid) = user_id {
            self.limiter.check_rate_limit(uid)
        } else {
            true
        };

        let fut = self.service.call(req);

        Box::pin(async move {
            if should_check && !rate_limit_passed {
                // Rate limit exceeded - return 429
                let response = HttpResponse::TooManyRequests()
                    .json(serde_json::json!({
                        "success": false,
                        "error": {
                            "code": "RATE_LIMIT_EXCEEDED",
                            "message": "Rate limit exceeded. Please try again later."
                        }
                    }));

                // Create a dummy request for the response
                // This is a workaround for the type system
                return Err(actix_web::error::ErrorTooManyRequests("Rate limit exceeded"));
            }

            // Process request normally
            fut.await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimitMiddleware::new(10, Duration::from_secs(60));
        assert_eq!(limiter.limit, 10);
    }

    #[test]
    fn test_rate_limiter_default() {
        let limiter = RateLimitMiddleware::default();
        assert_eq!(limiter.limit, 100);
    }

    #[test]
    fn test_rate_limit_check() {
        let limiter = RateLimitMiddleware::new(3, Duration::from_secs(60));
        let user_id = Uuid::new_v4();

        // First 3 requests should pass
        assert!(limiter.check_rate_limit(user_id));
        assert!(limiter.check_rate_limit(user_id));
        assert!(limiter.check_rate_limit(user_id));

        // 4th request should fail
        assert!(!limiter.check_rate_limit(user_id));
    }

    #[test]
    fn test_rate_limit_reset() {
        let limiter = RateLimitMiddleware::new(2, Duration::from_millis(100));
        let user_id = Uuid::new_v4();

        // Use up the limit
        assert!(limiter.check_rate_limit(user_id));
        assert!(limiter.check_rate_limit(user_id));
        assert!(!limiter.check_rate_limit(user_id));

        // Wait for reset
        std::thread::sleep(Duration::from_millis(150));

        // Should be able to make requests again
        assert!(limiter.check_rate_limit(user_id));
    }

    #[test]
    fn test_rate_limit_different_users() {
        let limiter = RateLimitMiddleware::new(2, Duration::from_secs(60));
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // User 1 uses up their limit
        assert!(limiter.check_rate_limit(user1));
        assert!(limiter.check_rate_limit(user1));
        assert!(!limiter.check_rate_limit(user1));

        // User 2 should still have their full limit
        assert!(limiter.check_rate_limit(user2));
        assert!(limiter.check_rate_limit(user2));
        assert!(!limiter.check_rate_limit(user2));
    }
}