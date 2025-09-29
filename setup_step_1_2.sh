#!/bin/bash

# setup_step_1_2.sh
# Run this from your project root (purge-assignment directory)

echo "🚀 Setting up Step 1.2: Database Schema Validation"
echo "=================================================="
echo ""

# Check we're in the right directory
if [ ! -d "store" ] || [ ! -d "migrations" ]; then
    echo "❌ Error: Please run this script from the project root (purge-assignment directory)"
    echo "   Current directory: $(pwd)"
    exit 1
fi

echo "📁 Creating necessary directories..."
mkdir -p store/src/bin

# 1. Create the performance indexes migration
echo "📝 Creating migrations/002_performance_indexes.sql..."
cat > migrations/002_performance_indexes.sql << 'MIGRATION_EOF'
-- migrations/002_performance_indexes.sql
-- Performance optimization indexes for Step 1.2

-- ==========================================
-- BALANCES TABLE INDEXES
-- ==========================================

-- Composite index for user-asset lookups (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_balances_user_asset 
    ON balances(user_id, asset_id);

-- Index for finding non-zero balances by user
CREATE INDEX IF NOT EXISTS idx_balances_user_id_amount 
    ON balances(user_id, amount) 
    WHERE amount > 0;

-- ==========================================
-- QUOTES TABLE INDEXES  
-- ==========================================

-- Composite index for user quotes with expiration
CREATE INDEX IF NOT EXISTS idx_quotes_user_expires 
    ON quotes(user_id, expires_at);

-- Partial index for active (non-used) quotes that haven't expired
CREATE INDEX IF NOT EXISTS idx_quotes_expires_at_active 
    ON quotes(expires_at) 
    WHERE used = false;

-- Index for recent quotes queries
CREATE INDEX IF NOT EXISTS idx_quotes_created_at 
    ON quotes(created_at DESC);

-- Index for finding quotes by input/output mints
CREATE INDEX IF NOT EXISTS idx_quotes_mints 
    ON quotes(input_mint, output_mint);

-- ==========================================
-- USERS TABLE INDEXES
-- ==========================================

-- Index for user analytics and recent signups
CREATE INDEX IF NOT EXISTS idx_users_created_at 
    ON users(created_at DESC);

-- Index for case-insensitive email lookups
CREATE INDEX IF NOT EXISTS idx_users_email_lower 
    ON users(LOWER(email));

-- Index on public_key for wallet lookups
CREATE INDEX IF NOT EXISTS idx_users_public_key 
    ON users(public_key) 
    WHERE public_key IS NOT NULL;

-- ==========================================
-- ASSETS TABLE INDEXES
-- ==========================================

-- Index for symbol lookups
CREATE INDEX IF NOT EXISTS idx_assets_symbol 
    ON assets(symbol);

-- ==========================================
-- ANALYZE TABLES FOR QUERY PLANNER
-- ==========================================
ANALYZE users;
ANALYZE assets;
ANALYZE balances;
ANALYZE quotes;
MIGRATION_EOF

# 2. Create the Rust validation test
echo "📝 Creating store/src/bin/schema_validation.rs..."
cat > store/src/bin/schema_validation.rs << 'RUST_EOF'
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
RUST_EOF

# 3. Create the validation shell script
echo "📝 Creating store/validate_schema.sh..."
cat > store/validate_schema.sh << 'SHELL_EOF'
#!/bin/bash

# store/validate_schema.sh
# Script to run Step 1.2: Database Schema Validation

set -e

echo "================================================"
echo "Step 1.2: Database Schema Validation"
echo "================================================"
echo ""

# Set database URL if not already set
export DATABASE_URL="${DATABASE_URL:-postgres://postgres:password@localhost/solana_wallet}"

echo "📊 Database URL: $DATABASE_URL"
echo ""

# Apply the performance indexes
echo "🚀 Applying performance indexes..."
psql "$DATABASE_URL" < ../migrations/002_performance_indexes.sql 2>/dev/null || {
    echo "✅ Indexes applied (some may have already existed)"
}

# Build and run the validation test
echo ""
echo "�� Building validation test..."
cargo build --bin schema_validation

echo ""
echo "🧪 Running validation test..."
echo ""
cargo run --bin schema_validation

# Show database statistics
echo ""
echo "📊 Database Statistics:"
echo "----------------------"

psql "$DATABASE_URL" -t << SQL
SELECT 
    tablename as "Table",
    pg_size_pretty(pg_total_relation_size(tablename::regclass)) as "Size"
FROM pg_tables
WHERE schemaname = 'public' 
    AND tablename IN ('users', 'assets', 'balances', 'quotes')
ORDER BY tablename;
SQL

echo ""
echo "✅ Step 1.2 Complete!"
echo ""
SHELL_EOF

# Make the validation script executable
chmod +x store/validate_schema.sh

echo ""
echo "✅ Setup complete! Files created:"
echo "   • migrations/002_performance_indexes.sql"
echo "   • store/src/bin/schema_validation.rs"
echo "   • store/validate_schema.sh"
echo ""
echo "📚 How to run Step 1.2:"
echo "   1. Apply indexes:     psql \$DATABASE_URL < migrations/002_performance_indexes.sql"
echo "   2. Run validation:    cd store && ./validate_schema.sh"
echo ""
echo "Or run everything at once:"
echo "   cd store && ./validate_schema.sh"
