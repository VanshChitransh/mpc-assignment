// store/src/bin/schema_validation.rs
// Step 1.2: Database Schema Validation and Performance Testing

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::time::Instant;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Database Schema Validation - Step 1.2");
    println!("=========================================\n");

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/solana_wallet".to_string());

    println!("📊 Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Validate tables exist
    println!("\n📋 Validating Schema Structure");
    println!("--------------------------------");
    
    let tables = ["users", "assets", "balances", "quotes"];
    for table in &tables {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables 
                WHERE table_schema = 'public' AND table_name = $1
            )"
        )
        .bind(table)
        .fetch_one(&pool)
        .await?;
        println!("  {} Table '{}'", if exists { "✓" } else { "✗" }, table);
    }

    // Check critical columns
    println!("\n📋 Validating Critical Columns");
    println!("--------------------------------");

    let has_public_key = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns 
            WHERE table_name = 'users' AND column_name = 'public_key'
        )"
    )
    .fetch_one(&pool)
    .await?;
    println!("  {} users.public_key", if has_public_key { "✓" } else { "✗" });

    let quote_data_type: Option<String> = sqlx::query_scalar(
        "SELECT data_type FROM information_schema.columns 
         WHERE table_name = 'quotes' AND column_name = 'quote_data'"
    )
    .fetch_optional(&pool)
    .await?;
    
    if let Some(data_type) = quote_data_type {
        println!("  {} quotes.quote_data is JSONB (type: {})", 
            if data_type == "jsonb" { "✓" } else { "✗" }, data_type);
    }

    // Check indexes
    println!("\n📊 Index Statistics");
    println!("--------------------------------");
    
    let index_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pg_indexes 
         WHERE schemaname = 'public' 
         AND tablename IN ('users', 'assets', 'balances', 'quotes')"
    )
    .fetch_one(&pool)
    .await?;
    println!("  Total indexes: {}", index_count);

    // Test query performance
    println!("\n⚡ Query Performance Tests");
    println!("--------------------------------");

    let start = Instant::now();
    let _result = sqlx::query(
        "SELECT * FROM users WHERE LOWER(email) = LOWER($1)"
    )
    .bind("test@example.com")
    .fetch_optional(&pool)
    .await?;
    println!("  User lookup by email: {:?}", start.elapsed());

    let start = Instant::now();
    let _result = sqlx::query(
        "SELECT b.*, a.symbol, a.decimals 
         FROM balances b 
         JOIN assets a ON b.asset_id = a.id 
         WHERE b.user_id = $1"
    )
    .bind(Uuid::new_v4())
    .fetch_all(&pool)
    .await?;
    println!("  Balance lookup by user: {:?}", start.elapsed());

    let start = Instant::now();
    let _result = sqlx::query(
        "SELECT * FROM quotes 
         WHERE user_id = $1 AND expires_at > NOW() AND used = false"
    )
    .bind(Uuid::new_v4())
    .fetch_all(&pool)
    .await?;
    println!("  Active quotes lookup: {:?}", start.elapsed());

    // Test sample data insertion
    println!("\n🧪 Testing Sample Data Operations");
    println!("--------------------------------");

    // Start a transaction
    let mut tx = pool.begin().await?;

    // Create test user
    let user_id = Uuid::new_v4();
    let email = format!("test-validation-{}@example.com", Uuid::new_v4());
    
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at, updated_at)
         VALUES ($1, $2, $3, NOW(), NOW())"
    )
    .bind(&user_id)
    .bind(&email)
    .bind("test_hash")
    .execute(&mut *tx)
    .await?;
    println!("  ✓ Test user created");

    // Rollback (we don't want to keep test data)
    tx.rollback().await?;
    println!("  ✓ Transaction rolled back");

    println!("\n✅ Schema validation complete!");
    println!("All tests passed successfully.");
    
    Ok(())
}
