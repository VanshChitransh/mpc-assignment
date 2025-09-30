// ============================================================================
// COMPREHENSIVE FIX FOR STEP 3.1 COMPILATION ISSUES
// ============================================================================

// ----------------------------------------------------------------------------
// FIX 1: Update backend/src/main.rs - Add Clone to AppState
// ----------------------------------------------------------------------------

// File: backend/src/main.rs
// Find the AppState struct and add #[derive(Clone)]

#[derive(Clone)] // ← ADD THIS
pub struct AppState {
    pub db: PgPool,
    pub store: Store,
    pub jwt_auth: JwtAuth,
    pub mpc_client: services::mpc::MpcClient, // ← Use full path
    pub jupiter_client: services::jupiter::JupiterClient, // ← Use full path
    pub solana_blockchain: blockchain::SolanaBlockchain,
    pub solana_client: services::solana::SolanaClient, // ← Use full path
}

// ----------------------------------------------------------------------------
// FIX 2: Update backend/src/lib.rs - Fix module exports
// ----------------------------------------------------------------------------

// File: backend/src/lib.rs
// Replace entire file with:

pub mod services;
pub mod routes;
pub mod middleware;
pub mod models;
pub mod blockchain;
pub mod error;

// Re-export AppState
pub use crate::services::mpc::MpcClient;
pub use crate::services::jupiter::JupiterClient;
pub use crate::services::solana::SolanaClient;

// Re-export commonly used types for tests
pub mod test_exports {
    pub use crate::services::mpc::{MpcClient, MpcError, RetryConfig, LoadBalancingStrategy, ClusterStatus};
    pub use crate::services::jupiter::JupiterClient;
    pub use crate::services::solana::SolanaClient;
}

// ----------------------------------------------------------------------------
// FIX 3: Update backend/src/services/mod.rs - Ensure proper exports
// ----------------------------------------------------------------------------

// File: backend/src/services/mod.rs

pub mod mpc;
pub mod jupiter;
pub mod solana;
pub mod wallet_service;

// Re-export for convenience
pub use mpc::{MpcClient, MpcError, create_default_mpc_client};
pub use jupiter::{JupiterClient, JupiterError, create_jupiter_client};
pub use solana::{SolanaClient, SolanaError, create_solana_client};
pub use wallet_service::{WalletService, WalletError};

// ----------------------------------------------------------------------------
// FIX 4: Update test file imports
// ----------------------------------------------------------------------------

// File: backend/tests/test_step_3_1_complete.rs
// Update the imports section at the top:

// REPLACE THIS:
// use backend::services::mpc::{...};

// WITH THIS:
use backend::test_exports::{
    MpcClient, MpcError, RetryConfig, LoadBalancingStrategy, ClusterStatus,
};

// ----------------------------------------------------------------------------
// FIX 5: Make request_timeout public in MpcClient
// ----------------------------------------------------------------------------

// File: backend/src/services/mpc.rs
// Find the MpcClient struct and change request_timeout visibility:

#[derive(Clone)]
pub struct MpcClient {
    client: Client,
    nodes: Vec<String>,
    threshold: u32,
    pub request_timeout: Duration, // ← ADD pub here
    load_balancer: Arc<LoadBalancer>,
    retry_config: RetryConfig,
}

// ----------------------------------------------------------------------------
// FIX 6: Fix Store initialization in main.rs
// ----------------------------------------------------------------------------

// File: backend/src/main.rs
// Replace the store initialization section:

// BEFORE:
// let store = match Store::from_url(&database_url).await {

// AFTER:
let store = match Store::new_pool(&database_url).await {
    Ok(pool) => {
        info!("Database pool created");
        Store::new(pool)
    }
    Err(e) => {
        error!("Failed to create database pool: {}", e);
        std::process::exit(1);
    }
};

// ----------------------------------------------------------------------------
// FIX 7: Update route imports to use correct paths
// ----------------------------------------------------------------------------

// File: backend/src/routes/wallet.rs
// Update imports at the top:

use crate::AppState;
use crate::services::wallet_service::{
    WalletService, WalletError, 
    KeyGenRequest, SignPhase1Request, SignPhase2Request, AggregateRequest
};
use actix_web::{web, HttpRequest, HttpResponse, Result, HttpMessage};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

// ----------------------------------------------------------------------------
// FIX 8: Update AppState usage in routes
// ----------------------------------------------------------------------------

// File: backend/src/routes/wallet.rs
// In each route handler, clone the services properly:

pub async fn keygen(
    data: web::Data<AppState>,
    req: HttpRequest,
    req_body: web::Json<KeyGenRequest>,
) -> Result<HttpResponse> {
    let user_id = *req.extensions().get::<Uuid>().unwrap();
    let keygen_req = req_body.into_inner();
    
    info!("Processing keygen request for user {}", user_id);
    
    // Clone the services from AppState
    let wallet_service = WalletService::new(
        data.mpc_client.clone(), // ← This should work now
        data.store.clone()       // ← This should work now
    );
    
    // Rest of the function...
}

// ----------------------------------------------------------------------------
// FIX 9: Ensure Store implements Clone
// ----------------------------------------------------------------------------

// File: store/src/lib.rs
// Add Clone derive to Store struct:

#[derive(Clone)] // ← ADD THIS
pub struct Store {
    pool: PgPool,
}

// ----------------------------------------------------------------------------
// FIX 10: Fix service creation functions in main.rs
// ----------------------------------------------------------------------------

// File: backend/src/main.rs
// Update service creation to use correct module paths:

use services::mpc::create_default_mpc_client;
use services::jupiter::create_jupiter_client;
use services::solana::create_solana_client;

// Then in main():
let mpc_client = create_default_mpc_client();
let jupiter_client = create_jupiter_client();
let solana_client = create_solana_client();
let solana_blockchain = blockchain::create_solana_blockchain();

// ----------------------------------------------------------------------------
// SUMMARY OF ALL CHANGES NEEDED
// ----------------------------------------------------------------------------

/*
1. ✅ Add #[derive(Clone)] to AppState in main.rs
2. ✅ Update lib.rs with test_exports module
3. ✅ Fix services/mod.rs exports
4. ✅ Update test imports to use test_exports
5. ✅ Make request_timeout public in MpcClient
6. ✅ Fix Store initialization in main.rs
7. ✅ Update route imports
8. ✅ Fix AppState usage in routes
9. ✅ Add Clone derive to Store
10. ✅ Fix service creation function imports
*/

// ----------------------------------------------------------------------------
// QUICK VERIFICATION CHECKLIST
// ----------------------------------------------------------------------------

/*
After applying fixes, verify:

1. cargo check - Should show no errors (warnings OK)
2. cargo build - Should compile successfully
3. cargo test --test test_step_3_1_complete --no-run - Should compile tests
4. cargo test --lib - Should run library tests

If you still have errors:
- Check that all imports use full module paths
- Verify Clone is derived on Store, AppState, and all services
- Ensure public visibility on fields accessed by tests
*/