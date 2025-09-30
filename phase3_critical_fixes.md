# Phase 3 Critical Fixes - Implementation Guide

## Current Status Analysis

### ✅ Already Working
- Route structure is correct (`/api/user/signup`, `/api/user/signin`, `/api/user/profile`)
- Auth middleware exists and has proper path validation
- Database schema and store layer are functional
- MPC cluster infrastructure is in place

### ❌ Critical Issues to Fix

#### 1. AuthMiddleware Not Applied (BLOCKING)
**File:** `backend/src/main.rs`
**Problem:** AuthMiddleware is defined but not applied to HttpServer
**Impact:** No authentication enforcement on protected endpoints

#### 2. Rate Limiting is Mock Implementation (HIGH)
**File:** `backend/src/middleware/rate_limit.rs`
**Problem:** Just passes through requests without any limiting
**Impact:** No protection against DDoS or abuse

#### 3. MPC TSS is Mock Implementation (CRITICAL)
**File:** `mpc/src/tss.rs`
**Problem:** Signatures are fake, not using real FROST protocol
**Impact:** Wallets are not actually secure

#### 4. Test Scripts Return Hardcoded Success (MEDIUM)
**Files:** Various test scripts
**Problem:** Tests always pass, don't validate actual behavior
**Impact:** False confidence in system correctness

## Implementation Order

### Phase 1: Quick Wins (30 minutes)
1. Apply AuthMiddleware to main.rs
2. Fix rate limiting middleware
3. Add health check endpoint

### Phase 2: MPC Implementation (2-3 hours)
1. Add FROST dependencies
2. Implement real key generation
3. Implement real signing protocol
4. Update MPC client in backend

### Phase 3: Testing & Validation (1 hour)
1. Fix test scripts
2. Run integration tests
3. Verify all endpoints

## Detailed Implementation Steps

### Step 1.1: Apply AuthMiddleware
```rust
// backend/src/main.rs - Line 14
use middleware::auth::{JwtAuth, AuthMiddleware};  // Add AuthMiddleware
use middleware::rate_limit::RateLimitMiddleware;  // Add this

// Line 64-76 - Update HttpServer configuration
HttpServer::new(move || {
    let cors = Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);

    App::new()
        .app_data(web::Data::new(app_state.clone()))
        .wrap(RateLimitMiddleware::new())  // Add this FIRST
        .wrap(AuthMiddleware::new(app_state.jwt_auth.clone()))  // Add this SECOND
        .wrap(cors)
        .wrap(Logger::default())
        .configure(routes::user::config)
        .configure(routes::solana::config)
        .service(web::scope("/api").configure(routes::api::config))
})
```

### Step 1.2: Implement Real Rate Limiting
See full implementation in rate_limit.rs below.

### Step 1.3: Add Health Check
```rust
// backend/src/routes/mod.rs
pub mod health;

// backend/src/routes/health.rs (new file)
use actix_web::{web, HttpResponse, Responder};

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub fn config(cfg: &web::ServiceConfig) {
    cfg.service(
        web::resource("/health")
            .route(web::get().to(health_check))
    );
}
```

### Step 2: FROST MPC Implementation
This is complex - requires full rewrite of `mpc/src/tss.rs`.
Key changes:
1. Add `frost-ed25519 = "2.0.0"` to `mpc/Cargo.toml`
2. Replace mock functions with real FROST protocol
3. Implement proper key generation, signing rounds, aggregation

### Step 3: Fix Test Scripts
Update scripts to:
1. Capture actual HTTP responses
2. Parse JSON and validate fields
3. Return proper exit codes
4. Track pass/fail for each test

## Testing After Fixes

```bash
# Terminal 1: Start backend
cd backend
cargo run

# Terminal 2: Test authentication
# Should fail without token
curl -v http://localhost:8080/api/user/profile
# Expected: 401 Unauthorized

# Signup
curl -X POST http://localhost:8080/api/user/signup \
  -H "Content-Type: application/json" \
  -d '{
    "username": "test@example.com",
    "email": "test@example.com",
    "password": "SecurePass123!"
  }'
# Expected: 200 OK with user details

# Signin and get token
TOKEN=$(curl -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{
    "username": "test@example.com",
    "password": "SecurePass123!"
  }' | jq -r '.token')

# Access protected endpoint
curl http://localhost:8080/api/user/profile \
  -H "Authorization: Bearer $TOKEN"
# Expected: 200 OK with user profile

# Test rate limiting (run 101 times)
for i in {1..101}; do
  curl -s http://localhost:8080/api/user/profile \
    -H "Authorization: Bearer $TOKEN" \
    -o /dev/null \
    -w "%{http_code}\n"
done | tail -1
# Expected: Last response should be 429 (Too Many Requests)
```

## Success Criteria

- [ ] Auth middleware blocks unauthenticated requests
- [ ] Signup creates users successfully
- [ ] Signin returns valid JWT tokens
- [ ] Protected endpoints require valid tokens
- [ ] Rate limiting kicks in after 100 requests/minute
- [ ] Health check endpoint returns 200
- [ ] All test scripts return accurate pass/fail results

## Time Estimates

- Auth middleware: 5 minutes
- Rate limiting: 10 minutes
- Health check: 5 minutes
- FROST implementation: 2-3 hours
- Test script fixes: 30 minutes
- Integration testing: 30 minutes

**Total: 4-5 hours for complete Phase 3**
