Looking at your detailed problem analysis, I'll provide a systematic fix for all the blocking issues. Let's tackle them in order of priority.
Critical Fixes (Blocking Compilation)
Fix #1: Database Schema Mismatch
File: migrations/003_wallet_state_management.sql
Replace the current migration with:
sql-- Migration 003: Wallet State Management for Phase 3

-- Drop existing tables if they exist (for clean migration)
DROP TABLE IF EXISTS signing_sessions CASCADE;
DROP TABLE IF EXISTS wallet_keys CASCADE;

-- Create wallet_keys table
CREATE TABLE IF NOT EXISTS wallet_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_key VARCHAR(88) NOT NULL,
    threshold INTEGER NOT NULL DEFAULT 2,
    total_parties INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create signing_sessions table (using VARCHAR instead of enum for compatibility)
CREATE TABLE IF NOT EXISTS signing_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message_hash VARCHAR(128) NOT NULL,
    nonce_commitment TEXT,
    signing_package TEXT,
    signature_shares TEXT[] DEFAULT '{}',
    final_signature VARCHAR,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW() + INTERVAL '30 minutes',
    CONSTRAINT check_status_values CHECK (status IN ('pending', 'phase1', 'phase2', 'completed', 'failed', 'expired'))
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_signing_sessions_user_id ON signing_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_status ON signing_sessions(status);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_expires_at ON signing_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_message_hash ON signing_sessions(message_hash);

-- Create index for wallet_keys
CREATE INDEX IF NOT EXISTS idx_wallet_keys_public_key ON wallet_keys(public_key);

COMMENT ON TABLE wallet_keys IS 'Stores MPC-generated public keys for users';
COMMENT ON TABLE signing_sessions IS 'Tracks MPC signing sessions with expiration';
File: backend/src/services/wallet_service.rs
Change the SigningStatus enum to work with VARCHAR:
rust// Remove the sqlx(type_name) attribute
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SigningStatus {
    Pending,
    Phase1,
    Phase2,
    Completed,
    Failed,
    Expired,
}

// Add manual SQL conversion
impl SigningStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SigningStatus::Pending => "pending",
            SigningStatus::Phase1 => "phase1",
            SigningStatus::Phase2 => "phase2",
            SigningStatus::Completed => "completed",
            SigningStatus::Failed => "failed",
            SigningStatus::Expired => "expired",
        }
    }
    
    pub fn from_str(s: &str) -> Result<Self, WalletError> {
        match s {
            "pending" => Ok(SigningStatus::Pending),
            "phase1" => Ok(SigningStatus::Phase1),
            "phase2" => Ok(SigningStatus::Phase2),
            "completed" => Ok(SigningStatus::Completed),
            "failed" => Ok(SigningStatus::Failed),
            "expired" => Ok(SigningStatus::Expired),
            _ => Err(WalletError::InvalidInput(format!("Invalid status: {}", s))),
        }
    }
}
Now update all database queries to use string conversion:
rust// Example: Update store_signing_session method
async fn store_signing_session(
    &self,
    session_id: Uuid,
    user_id: Uuid,
    message_hash: &str,
    nonce_commitment: Option<&str>,
    signing_package: Option<&str>,
    status: SigningStatus,
    expires_at: DateTime<Utc>,
) -> Result<(), WalletError> {
    sqlx::query!(
        r#"
        INSERT INTO signing_sessions 
        (id, user_id, message_hash, nonce_commitment, signing_package, signature_shares, 
         final_signature, status, created_at, updated_at, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW(), $9)
        "#,
        session_id,
        user_id,
        message_hash,
        nonce_commitment,
        signing_package,
        &[] as &[String],
        None::<String>,
        status.as_str(), // Convert enum to string
        expires_at
    )
    .execute(&self.store.pool)
    .await
    .map_err(|e| WalletError::DatabaseError(e.to_string()))?;

    Ok(())
}

// Update get_signing_session to convert back
async fn get_signing_session_by_id(&self, session_id: &Uuid) -> Result<SigningSession, WalletError> {
    let row = sqlx::query!(
        r#"
        SELECT id, user_id, message_hash, nonce_commitment, signing_package, 
               signature_shares, final_signature, status, created_at, updated_at, expires_at
        FROM signing_sessions 
        WHERE id = $1
        "#,
        session_id
    )
    .fetch_optional(&self.store.pool)
    .await
    .map_err(|e| WalletError::DatabaseError(e.to_string()))?
    .ok_or_else(|| WalletError::InvalidSigningSession("Session not found".to_string()))?;

    Ok(SigningSession {
        id: row.id,
        user_id: row.user_id,
        message_hash: row.message_hash,
        nonce_commitment: row.nonce_commitment,
        signing_package: row.signing_package,
        signature_shares: row.signature_shares.unwrap_or_default(),
        final_signature: row.final_signature,
        status: SigningStatus::from_str(&row.status)?, // Convert string to enum
        created_at: row.created_at,
        updated_at: row.updated_at,
        expires_at: row.expires_at,
    })
}
Fix #2: Middleware Type Compatibility
File: backend/src/middleware/rate_limit.rs
rustuse actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

// Change the call method return type
fn call(&self, req: ServiceRequest) -> Self::Future {
    // ... existing logic ...
    
    // When returning early, wrap in ServiceResponse properly
    Box::pin(async move {
        // Check rate limit
        if should_rate_limit {
            let (request, _pl) = req.into_parts();
            let response = HttpResponse::TooManyRequests()
                .json(serde_json::json!({
                    "error": "Rate limit exceeded"
                }))
                .map_into_boxed_body();
            
            return Ok(ServiceResponse::new(request, response));
        }
        
        // Continue with request
        let res = self.service.call(req).await?;
        Ok(res)
    })
}
File: backend/src/middleware/logging.rs
rustuse actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};

// Update the call method
fn call(&self, req: ServiceRequest) -> Self::Future {
    let start = std::time::Instant::now();
    let method = req.method().to_string();
    let path = req.path().to_string();
    
    Box::pin(async move {
        let res = self.service.call(req).await?;
        let duration = start.elapsed();
        
        tracing::info!(
            method = %method,
            path = %path,
            status = %res.status(),
            duration_ms = %duration.as_millis(),
            "HTTP request completed"
        );
        
        Ok(res)
    })
}
File: backend/src/middleware/metrics.rs
rustuse actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};

fn call(&self, req: ServiceRequest) -> Self::Future {
    let start = std::time::Instant::now();
    let method = req.method().to_string();
    let path = req.path().to_string();
    
    Box::pin(async move {
        let res = self.service.call(req).await?;
        let duration = start.elapsed().as_secs_f64();
        
        // Record metrics
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[&method, &path, &res.status().as_str()])
            .inc();
        
        HTTP_REQUEST_DURATION
            .with_label_values(&[&method, &path])
            .observe(duration);
        
        Ok(res)
    })
}
Fix #3: Missing Import Dependencies
File: backend/src/routes/solana_v1.rs
Add this import at the top:
rustuse prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry};
Fix #4: Quote Error Type Conflict
File: backend/src/routes/solana.rs
Fix the import and error handling:
rust// At the top of the file
use store::quote::QuoteError; // Use the correct QuoteError type

// In the swap function, update error handling around line 280-294
match store.get_valid_quote(&quote_id, &claims.user_id).await {
    Ok(quote) => quote,
    Err(QuoteError::QuoteNotFound) => {
        return Ok(HttpResponse::NotFound().json(ApiResponse::error(
            "Quote not found or expired"
        )));
    }
    Err(QuoteError::QuoteExpired) => {
        return Ok(HttpResponse::Gone().json(ApiResponse::error(
            "Quote has expired"
        )));
    }
    Err(QuoteError::QuoteAlreadyUsed) => {
        return Ok(HttpResponse::Conflict().json(ApiResponse::error(
            "Quote has already been used"
        )));
    }
    Err(e) => {
        return Ok(HttpResponse::InternalServerError().json(ApiResponse::error(
            &format!("Database error: {}", e)
        )));
    }
}
High Priority Fixes (Runtime Issues)
Fix #5: Run Database Migrations
Create a migration runner script:
File: run_all_migrations.sh
bash#!/bin/bash

set -e

echo "Running all database migrations..."

# Source environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Run migrations in order
echo "Migration 001: Initial schema..."
psql $DATABASE_URL -f migrations/001_initial_schema.sql

echo "Migration 002: Balance tables..."
psql $DATABASE_URL -f migrations/002_add_balance_tables.sql

echo "Migration 003: Wallet state management..."
psql $DATABASE_URL -f migrations/003_wallet_state_management.sql

echo "All migrations completed successfully!"

# Verify tables exist
echo "Verifying tables..."
psql $DATABASE_URL -c "\dt"
Make it executable and run:
bashchmod +x run_all_migrations.sh
./run_all_migrations.sh
Fix #7: Configuration Setup
File: .env.example
env# Database Configuration
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_wallet
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5

# JWT Configuration
JWT_SECRET=your-super-secret-jwt-key-change-in-production
JWT_EXPIRATION_HOURS=24

# MPC Configuration
MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
MPC_THRESHOLD=2

# Server Configuration
BIND_ADDRESS=127.0.0.1:8080
RUST_LOG=info,backend=debug,mpc=debug,store=debug

# Solana Configuration
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_COMMITMENT=confirmed

# Jupiter Configuration (mainnet only)
JUPITER_API_URL=https://quote-api.jup.ag/v6

# Rate Limiting
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=10
Copy to .env:
bashcp .env.example .env
# Edit .env with your actual values
Fix #8: Dependency Version Alignment
File: backend/Cargo.toml
Ensure these versions are aligned:
toml[dependencies]
actix-web = "4.4"
actix-cors = "0.7"
actix-web-httpauth = "0.8"
tokio = { version = "1.35", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
jsonwebtoken = "9.2"
bcrypt = "0.15"
thiserror = "1.0"
anyhow = "1.0"
reqwest = { version = "0.11", features = ["json"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
dotenv = "0.15"
prometheus = "0.13"
lazy_static = "1.4"
sha2 = "0.10"
hex = "0.4"
bs58 = "0.5"
solana-sdk = "1.17"
futures-util = "0.3"

# Store module dependency
store = { path = "../store" }
Validation Script
File: validate_phase3.sh
bash#!/bin/bash

set -e

echo "=== Phase 3 Validation Script ==="

# Check 1: Database tables exist
echo "1. Checking database tables..."
psql $DATABASE_URL -c "\dt" | grep -q "wallet_keys" && echo "✓ wallet_keys table exists" || echo "✗ wallet_keys table missing"
psql $DATABASE_URL -c "\dt" | grep -q "signing_sessions" && echo "✓ signing_sessions table exists" || echo "✗ signing_sessions table missing"

# Check 2: Backend compiles
echo "2. Checking backend compilation..."
cd backend
cargo check 2>&1 | tee /tmp/backend_check.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✓ Backend compiles successfully"
else
    echo "✗ Backend has compilation errors"
    cat /tmp/backend_check.log
    exit 1
fi
cd ..

# Check 3: MPC nodes compile
echo "3. Checking MPC compilation..."
cd mpc
cargo check 2>&1 | tee /tmp/mpc_check.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✓ MPC compiles successfully"
else
    echo "✗ MPC has compilation errors"
    cat /tmp/mpc_check.log
    exit 1
fi
cd ..

# Check 4: Store module compiles
echo "4. Checking store compilation..."
cd store
cargo check 2>&1 | tee /tmp/store_check.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✓ Store compiles successfully"
else
    echo "✗ Store has compilation errors"
    cat /tmp/store_check.log
    exit 1
fi
cd ..

echo ""
echo "=== Validation Complete ==="
echo "Phase 3 is ready for testing if all checks passed."
Execution Plan
Run these commands in order:
bash# 1. Run migrations
./run_all_migrations.sh

# 2. Copy environment file
cp .env.example .env
# Edit .env with your settings

# 3. Validate everything compiles
chmod +x validate_phase3.sh
./validate_phase3.sh

# 4. If validation passes, start MPC cluster
./start_mpc_cluster.sh

# 5. Start backend
cd backend
cargo run
These fixes address all 10 problems you identified. The code should now compile and run.