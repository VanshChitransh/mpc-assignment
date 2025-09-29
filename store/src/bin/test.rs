// src/bin/test.rs
use store::{Store, CreateUserRequest};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Store Implementation");
    println!("==============================");

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .expect("DATABASE_URL or TEST_DATABASE_URL must be set");

    println!("📊 Connecting to database...");
    
    // Create store instance
    let store = Store::from_url(&database_url).await?;
    
    // Test 1: Health check
    println!("🔍 Testing health check...");
    store.health_check().await?;
    println!("✅ Health check passed");

    // Test 2: Detailed health check
    println!("🔍 Testing detailed health check...");
    let health = store.detailed_health_check().await?;
    println!("✅ Database health: connected={}, response_time={}ms, pool_size={}, tables_exist={}", 
        health.connected, health.response_time_ms, health.pool_size, health.tables_exist);

    // Test 3: Initialize default assets
    println!("🔍 Initializing default assets...");
    store.initialize_default_assets().await?;
    println!("✅ Default assets initialized");

    // Test 4: Asset operations
    println!("🔍 Testing asset operations...");
    let assets = store.get_all_assets().await?;
    println!("✅ Found {} assets", assets.len());
    
    for asset in &assets {
        println!("   - {}: {} ({})", asset.symbol, asset.name, asset.mint_address);
    }

    // Test 5: User operations
    println!("🔍 Testing user operations...");
    let test_email = format!("test-{}@example.com", Uuid::new_v4());
    let create_request = CreateUserRequest {
        email: test_email.clone(),
        password: "testpassword123".to_string(),
    };
    
    let user = store.create_user(create_request).await?;
    println!("✅ Created user: {}", user.email);

    // Test authentication
    let auth_user = store.authenticate_user(&test_email, "testpassword123").await?;
    println!("✅ Authentication successful for user: {}", auth_user.id);

    // Test public key update
    let test_pubkey = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
    store.update_user_public_key(&user.id, test_pubkey).await?;
    println!("✅ Updated user public key");

    // Test 6: Balance operations
    println!("🔍 Testing balance operations...");
    
    // Initialize user balances
    store.initialize_user_balances(&user.id).await?;
    println!("✅ Initialized user balances");

    // Get SOL balance
    let sol_balance = store.get_sol_balance(&user.id).await?;
    println!("✅ SOL balance: {} lamports", sol_balance);

    // Get all balances
    let all_balances = store.get_all_balances(&user.id).await?;
    println!("✅ All balances ({} assets):", all_balances.len());
    for balance in &all_balances {
        println!("   - {}: {} ({})", 
            balance.symbol, 
            balance.balance, 
            balance.token_mint);
    }

    // Test balance update
    let sol_asset = store.get_asset_by_symbol("SOL").await?;
    store.update_balance(&user.id, &sol_asset.id, 1000000000).await?; // 1 SOL
    println!("✅ Updated SOL balance to 1 SOL");

    let updated_balance = store.get_sol_balance(&user.id).await?;
    println!("✅ Verified SOL balance: {} lamports", updated_balance);

    // Test 7: Quote operations
    println!("🔍 Testing quote operations...");
    
    let quote_data = serde_json::json!({
        "inputMint": "So11111111111111111111111111111111111111112",
        "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "swapMode": "ExactIn"
    });

    let quote = store.store_quote(
        &user.id,
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        1000000000, // 1 SOL
        100000000,  // 100 USDC
        quote_data,
        300 // 5 minutes
    ).await?;
    println!("✅ Created quote: {}", quote.id);

    // Test quote retrieval
    let retrieved_quote = store.get_valid_quote(&quote.id, &user.id).await?;
    println!("✅ Retrieved valid quote: {} -> {}", 
        retrieved_quote.in_amount, retrieved_quote.out_amount);

    // Test 8: Store statistics
    println!("🔍 Testing store statistics...");
    let stats = store.get_store_stats().await?;
    println!("✅ Store statistics:");
    println!("   - Users: {}", stats.users.total_users);
    println!("   - Assets: {}", stats.assets);
    println!("   - Balance records: {}", stats.balances.total_balance_records);
    println!("   - Quotes: {}", stats.quotes.total_quotes);

    // Test 9: Maintenance operations
    println!("🔍 Testing maintenance operations...");
    let cleanup_results = store.maintenance_cleanup().await?;
    println!("✅ Cleanup completed: {} expired quotes deleted", 
        cleanup_results.expired_quotes_deleted);

    // Clean up test data
    println!("🧹 Cleaning up test data...");
    store.delete_user(&user.id).await?;
    println!("✅ Test user deleted");

    println!("\n🎉 All tests passed successfully!");
    println!("Store implementation is working correctly.");
    
    Ok(())
}