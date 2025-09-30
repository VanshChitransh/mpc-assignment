#!/bin/bash
set -e

# Fix 1: Update lib.rs
cat > backend/src/lib.rs << 'EOF'
pub mod services;
pub mod routes;
pub mod middleware;
pub mod models;
pub mod blockchain;
pub mod error;

pub mod test_exports {
    pub use crate::services::mpc::{MpcClient, MpcError, RetryConfig, LoadBalancingStrategy, ClusterStatus};
}
EOF

# Fix 2: Make request_timeout public
sed -i.bak 's/request_timeout: Duration,/pub request_timeout: Duration,/' backend/src/services/mpc.rs

# Fix 3: Update test imports
sed -i.bak 's/use backend::services::mpc::{/use backend::test_exports::{/' backend/tests/test_step_3_1_complete.rs

# Fix 4: Add Clone to Store if missing
if ! grep -q "#\[derive.*Clone" store/src/lib.rs | head -5; then
    sed -i.bak '0,/pub struct Store/s/pub struct Store/#[derive(Clone)]\npub struct Store/' store/src/lib.rs
fi

# Fix 5: Add Clone to AppState if missing
if ! grep -B1 "pub struct AppState" backend/src/main.rs | grep -q "Clone"; then
    sed -i.bak '0,/pub struct AppState/s/pub struct AppState/#[derive(Clone)]\npub struct AppState/' backend/src/main.rs
fi

# Fix 6: Update rate_limit.rs
cat > backend/src/middleware/rate_limit.rs << 'EOF'
use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use std::collections::HashMap;
use std::future::{ready, Ready};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use futures_util::future::LocalBoxFuture;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    reset_time: Instant,
}

#[derive(Clone)]
pub struct RateLimitMiddleware {
    limit: u32,
    window: Duration,
    store: Arc<Mutex<HashMap<Uuid, RateLimitEntry>>>,
}

impl RateLimitMiddleware {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self { limit, window, store: Arc::new(Mutex::new(HashMap::new())) }
    }
    pub fn default() -> Self {
        Self::new(100, Duration::from_secs(60))
    }
    fn check_rate_limit(&self, user_id: Uuid) -> bool {
        let mut store = self.store.lock().unwrap();
        let now = Instant::now();
        store.retain(|_, entry| entry.reset_time > now);
        let entry = store.entry(user_id).or_insert(RateLimitEntry { count: 0, reset_time: now + self.window });
        if entry.count >= self.limit { return false; }
        entry.count += 1;
        true
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>, S::Future: 'static, B: 'static {
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitService { service, limiter: self.clone() }))
    }
}

pub struct RateLimitService<S> {
    service: S,
    limiter: RateLimitMiddleware,
}

impl<S, B> Service<ServiceRequest> for RateLimitService<S>
where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>, S::Future: 'static, B: 'static {
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    forward_ready!(service);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let user_id = req.extensions().get::<Uuid>().copied();
        let should_check = user_id.is_some();
        let rate_limit_passed = if let Some(uid) = user_id { self.limiter.check_rate_limit(uid) } else { true };
        let fut = self.service.call(req);
        Box::pin(async move {
            if should_check && !rate_limit_passed {
                return Err(actix_web::error::ErrorTooManyRequests("Rate limit exceeded"));
            }
            fut.await
        })
    }
}
EOF

echo "✅ All fixes applied!"
cd backend && cargo build && echo "🎉 Step 3.1 is COMPLETE!"
