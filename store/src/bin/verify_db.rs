// Save this as: store/src/bin/verify_db.rs
use store::{Store, CreateUserRequest};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Verifying Database Setup");
    println!("===========================");

    // Load from environment
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    println!("📊 Connecting to database...");
    let store = Store::from_url(&database_url).await?;
    
    // Test 1: Basic health check
    println!("\n1️⃣ Testing basic health check...");
    store.health_check().await?;
    println!("   ✅ Database connection successful");

    // Test 2: Check tables exist
    println!("\n2️⃣ Checking tables...");
    let health = store.detailed_health_check().await?;
    println!("   ✅ Tables exist: {}", health.tables_exist);
    println!("   ✅ Response time: {}ms", health.response_time_ms);

    // Test 3: Check default assets
    println!("\n3️⃣ Checking default assets...");
    let assets = store.get_all_assets().await?;
    println!("   ✅ Found {} assets:", assets.len());
    for asset in &assets {
        println!("      - {}: {} ({})", asset.symbol, asset.name, asset.mint_address);
    }

    // Test 4: Create a test user
    println!("\n4️⃣ Testing user creation...");
    let test_email = format!("verify-{}@test.com", Uuid::new_v4());
    let user = store.create_user(CreateUserRequest {
        email: test_email.clone(),
        password: "testpass123".to_string(),
    }).await?;
    println!("   ✅ Created user: {}", user.email);

    // Test 5: Test authentication
    println!("\n5️⃣ Testing authentication...");
    let _auth_user = store.authenticate_user(&test_email, "testpass123").await?;
    println!("   ✅ Authentication successful");

    // Test 6: Initialize balances
    println!("\n6️⃣ Testing balance initialization...");
    store.initialize_user_balances(&user.id).await?;
    let sol_balance = store.get_sol_balance(&user.id).await?;
    println!("   ✅ Initialized balances, SOL balance: {}", sol_balance);

    // Test 7: Test quote storage
    println!("\n7️⃣ Testing quote storage...");
    let quote_data = serde_json::json!({
        "test": "data",
        "inputMint": "SOL",
        "outputMint": "USDC"
    });
    let quote = store.store_quote(
        &user.id,
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        1000000000,
        100000000,
        quote_data,
        300
    ).await?;
    println!("   ✅ Created quote: {}", quote.id);

    // Clean up
    println!("\n🧹 Cleaning up test data...");
    store.delete_user(&user.id).await?;
    println!("   ✅ Test user deleted");

    println!("\n✅ All database operations working correctly!");
    println!("🎉 Your database is properly set up and ready to use!");
    
    Ok(())
}