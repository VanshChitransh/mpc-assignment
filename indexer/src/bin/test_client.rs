use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("Starting database test client...");
    
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/solana_indexer".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    info!("Connected to database successfully");
    
    // Test 1: Create a test user (without username)
    info!("Test 1: Creating test user");
    let user_id = sqlx::query_scalar!(
        "INSERT INTO users (email) VALUES ($1) RETURNING id",
        "test@example.com"
    )
    .fetch_one(&pool)
    .await;
    
    match user_id {
        Ok(id) => info!("✓ Created user with ID: {}", id),
        Err(e) => {
            if e.to_string().contains("duplicate key") {
                info!("✓ User already exists (expected)");
                // Get existing user
                let existing_id = sqlx::query_scalar!(
                    "SELECT id FROM users WHERE email = $1",
                    "test@example.com"
                )
                .fetch_one(&pool)
                .await?;
                info!("✓ Found existing user ID: {}", existing_id);
            } else {
                error!("✗ Failed to create user: {}", e);
                return Err(e.into());
            }
        }
    }
    
    // Test 2: Add a test wallet
    info!("Test 2: Adding test wallet");
    let test_address = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC mint for testing
    
    let user_id = sqlx::query_scalar!(
        "SELECT id FROM users WHERE email = $1",
        "test@example.com"
    )
    .fetch_one(&pool)
    .await?;
    
    let wallet_result = sqlx::query!(
        r#"
        INSERT INTO user_wallets (user_id, address, sol_balance, is_active)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (address) DO UPDATE SET
            user_id = $1,
            sol_balance = $3,
            is_active = $4,
            updated_at = NOW()
        "#,
        user_id,
        test_address,
        1000000_i64, // 1 SOL in lamports
        true
    )
    .execute(&pool)
    .await?;
    
    info!("✓ Added test wallet, rows affected: {}", wallet_result.rows_affected());
    
    // Test 3: Record a balance change
    info!("Test 3: Recording balance change");
    let balance_change_result = sqlx::query!(
        r#"
        INSERT INTO balance_changes (address, old_balance, new_balance, slot, change_type)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        test_address,
        900000_i64,
        1000000_i64,
        150000000_i64, // Sample slot number
        "Transfer"
    )
    .execute(&pool)
    .await?;
    
    info!("✓ Recorded balance change, rows affected: {}", balance_change_result.rows_affected());
    
    // Test 4: Query data
    info!("Test 4: Querying database");
    
    let user_wallets = sqlx::query!(
        r#"
        SELECT 
            u.email,
            uw.address,
            uw.sol_balance,
            uw.is_active,
            uw.created_at
        FROM users u
        JOIN user_wallets uw ON u.id = uw.user_id
        WHERE uw.is_active = true
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    info!("✓ Found {} active wallets:", user_wallets.len());
    for wallet in user_wallets {
        info!("  - User: {}", wallet.email);
        info!("    Address: {}", wallet.address);
        info!("    Balance: {} lamports", wallet.sol_balance.unwrap_or(0));
        info!(
            "    Created: {}",
            wallet
                .created_at
                .map(|dt| dt.to_string())
                .unwrap_or("N/A".to_string())
        );
    }
    
    // Test 5: Check balance changes
    info!("Test 5: Checking balance changes");
    let recent_changes = sqlx::query!(
        r#"
        SELECT 
            address,
            old_balance,
            new_balance,
            slot,
            change_type,
            created_at
        FROM balance_changes
        WHERE created_at > NOW() - INTERVAL '1 hour'
        ORDER BY created_at DESC
        LIMIT 5
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    info!("✓ Found {} recent balance changes:", recent_changes.len());
    for change in recent_changes {
        info!("  - Address: {}", change.address);
        info!(
            "    Change: {} -> {} lamports",
            change.old_balance,
            change.new_balance
        );
        info!("    Type: {}, Slot: {}", change.change_type, change.slot);
        info!(
            "    Time: {}",
            change
                .created_at
                .map(|dt| dt.to_string())
                .unwrap_or("N/A".to_string())
        );
    }
    
    // Test 6: Record metrics
    info!("Test 6: Recording metrics");
    let metric_result = sqlx::query!(
        r#"
        INSERT INTO subscription_metrics (metric_name, metric_value, tags)
        VALUES ($1, $2, $3)
        "#,
        "test_metric",
        42_i64,
        serde_json::json!({"test": true, "client": "test_client"})
    )
    .execute(&pool)
    .await?;
    
    info!("✓ Recorded metric, rows affected: {}", metric_result.rows_affected());
    
    // Test 7: Update indexer state
    info!("Test 7: Updating indexer state");
    let state_result = sqlx::query!(
        r#"
        INSERT INTO indexer_state (key, value)
        VALUES ($1, $2)
        ON CONFLICT (key) DO UPDATE SET
            value = $2,
            updated_at = NOW()
        "#,
        "test_last_slot",
        "150000042"
    )
    .execute(&pool)
    .await?;
    
    info!("✓ Updated indexer state, rows affected: {}", state_result.rows_affected());
    
    // Test 8: Database statistics
    info!("Test 8: Database statistics");
    let stats = sqlx::query!(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM users) as user_count,
            (SELECT COUNT(*) FROM user_wallets WHERE is_active = true) as active_wallets,
            (SELECT COUNT(*) FROM balance_changes) as total_balance_changes,
            (SELECT COUNT(*) FROM subscription_metrics) as total_metrics,
            (SELECT COALESCE(SUM(sol_balance), 0) FROM user_wallets WHERE is_active = true) as total_balance
        "#
    )
    .fetch_one(&pool)
    .await?;
    
    info!("✓ Database Statistics:");
    info!("  - Users: {}", stats.user_count.unwrap_or(0));
    info!("  - Active wallets: {}", stats.active_wallets.unwrap_or(0));
    info!("  - Balance changes: {}", stats.total_balance_changes.unwrap_or(0));
    info!("  - Metrics recorded: {}", stats.total_metrics.unwrap_or(0));
    info!(
        "  - Total balance tracked: {} lamports",
        stats.total_balance
            .map(|v| v.to_string())
            .unwrap_or("0".to_string())
    );
    
    info!("✓ All database tests completed successfully!");
    info!("Database is ready for the indexer");
    
    Ok(())
}
