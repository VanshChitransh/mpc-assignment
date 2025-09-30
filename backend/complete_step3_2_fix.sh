#!/bin/bash

echo "🔧 Complete Fix for Step 3.2 - Backend User Routes with Authentication"
echo "====================================================================="

cd /Users/vansh/Coding/SuperDevs/Assignments/purge-assignment/backend

# 1. Create the migrations directory and migration file
echo "📁 Setting up migrations directory..."
mkdir -p migrations

echo "📝 Creating migration file..."
cat > migrations/001_initial_schema.sql << 'EOF'
-- migrations/001_initial_schema.sql
-- Initial schema for Solana wallet backend

-- Users table (updated to include public_key and proper timestamp handling)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR UNIQUE NOT NULL,
    password_hash VARCHAR NOT NULL,
    public_key VARCHAR, -- Solana public key from MPC
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Assets table (supported tokens)
CREATE TABLE IF NOT EXISTS assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mint_address VARCHAR UNIQUE NOT NULL, -- Solana mint address
    decimals INTEGER NOT NULL,
    name VARCHAR NOT NULL,
    symbol VARCHAR NOT NULL,
    logo_url VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Balances table (user token balances)
CREATE TABLE IF NOT EXISTS balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    amount BIGINT NOT NULL DEFAULT 0, -- stored in smallest units (lamports for SOL)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    UNIQUE(user_id, asset_id)
);

-- Quotes table (Jupiter swap quotes)
CREATE TABLE IF NOT EXISTS quotes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    input_mint VARCHAR NOT NULL,
    output_mint VARCHAR NOT NULL,
    in_amount BIGINT NOT NULL,
    out_amount BIGINT NOT NULL,
    quote_data JSONB NOT NULL, -- Full Jupiter quote response
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_public_key ON users(public_key);
CREATE INDEX IF NOT EXISTS idx_balances_user_id ON balances(user_id);
CREATE INDEX IF NOT EXISTS idx_balances_asset_id ON balances(asset_id);
CREATE INDEX IF NOT EXISTS idx_balances_user_asset ON balances(user_id, asset_id);
CREATE INDEX IF NOT EXISTS idx_quotes_user_id ON quotes(user_id);
CREATE INDEX IF NOT EXISTS idx_quotes_expires_at ON quotes(expires_at);
CREATE INDEX IF NOT EXISTS idx_quotes_used ON quotes(used);
CREATE INDEX IF NOT EXISTS idx_assets_mint_address ON assets(mint_address);

-- Insert default assets (SOL and USDC)
INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url) VALUES
    (gen_random_uuid(), 'So11111111111111111111111111111111111111112', 9, 'Solana', 'SOL', NULL),
    (gen_random_uuid(), 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 6, 'USD Coin', 'USDC', NULL),
    (gen_random_uuid(), 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 6, 'Tether USD', 'USDT', NULL)
ON CONFLICT (mint_address) DO NOTHING;

-- Add trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_assets_updated_at BEFORE UPDATE ON assets  
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_balances_updated_at BEFORE UPDATE ON balances
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
EOF

echo "✅ Migration file created"

# 2. Fix the main.rs file to remove migration and fix imports
echo "📝 Fixing main.rs..."
cat > src/main.rs << 'EOF'
use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use env_logger::Env;
use sqlx::PgPool;
use std::env;

mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Create database connection pool
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to create database pool");

    let bind_address = "127.0.0.1:8080";
    println!("🚀 Starting server at http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/user")
                            .service(routes::user::sign_up)
                            .service(routes::user::sign_in)
                            .service(routes::user::get_profile)
                    )
                    .service(
                        web::scope("/solana")
                            .service(routes::solana::get_balance)
                            .service(routes::solana::get_quote)
                            .service(routes::solana::execute_swap)
                            .service(routes::solana::send_tokens)
                    )
            )
            .service(routes::health::health_check)
    })
    .bind(bind_address)?
    .run()
    .await
}
EOF

echo "✅ main.rs fixed"

# 3. Create a simple migration runner
echo "📝 Creating migration runner script..."
cat > run_migrations.sh << 'EOF'
#!/bin/bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"

echo "🔄 Running database migrations manually..."
psql "$DATABASE_URL" -f migrations/001_initial_schema.sql

if [ $? -eq 0 ]; then
    echo "✅ Migrations completed successfully"
    echo "📊 Verifying schema..."
    psql "$DATABASE_URL" -c "\dt"
    echo ""
    echo "📊 Default assets:"
    psql "$DATABASE_URL" -c "SELECT name, symbol, decimals FROM assets;"
else
    echo "❌ Migration failed"
fi
EOF

chmod +x run_migrations.sh

# 4. Run the migration
echo "🔄 Running database migrations..."
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"

# Ensure database exists
createdb -h localhost -U postgres solana_wallet 2>/dev/null || echo "Database already exists"

# Run the migration
./run_migrations.sh

# 5. Test the build
echo "🧪 Testing build..."
if cargo build; then
    echo ""
    echo "🎉 SUCCESS! Step 3.2 Complete - Backend Setup"
    echo "============================================="
    echo ""
    echo "✅ What's working:"
    echo "   - Backend server with user authentication"
    echo "   - Database schema with proper tables"
    echo "   - All compilation errors fixed"
    echo ""
    echo "🚀 To start the server:"
    echo "   export DATABASE_URL=\"postgresql://postgres:postgres@localhost:5432/solana_wallet\""
    echo "   cargo run"
    echo ""
    echo "🧪 Test endpoints:"
    echo '   curl -X POST http://localhost:8080/api/user/signup -H "Content-Type: application/json" -d '\''{"email":"test@example.com","password":"password123"}'\'''
    echo '   curl http://localhost:8080/health'
    echo ""
    
else
    echo "❌ Build failed. Please check errors above."
    exit 1
fi