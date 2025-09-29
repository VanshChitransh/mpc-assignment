#!/bin/bash

# Complete database setup script for MPC Solana Wallet
# Run this from your project root directory

set -e  # Exit on error

echo "🚀 Complete Database Setup for MPC Solana Wallet"
echo "================================================="

# Configuration
DB_USER="postgres"
DB_PASS="password"
DB_HOST="localhost"
DB_PORT="5432"
DB_NAME="solana_wallet"

# You can customize these values:
read -p "Enter PostgreSQL user (default: postgres): " input_user
DB_USER=${input_user:-$DB_USER}

read -p "Enter PostgreSQL password (default: password): " input_pass
DB_PASS=${input_pass:-$DB_PASS}

read -p "Enter database name (default: solana_wallet): " input_dbname
DB_NAME=${input_dbname:-$DB_NAME}

DATABASE_URL="postgresql://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

echo ""
echo "📝 Configuration:"
echo "  DATABASE_URL: ${DATABASE_URL}"
echo ""

# Step 1: Check if PostgreSQL is running
echo "1️⃣ Checking PostgreSQL status..."
if pg_isready -h $DB_HOST -p $DB_PORT > /dev/null 2>&1; then
    echo "   ✅ PostgreSQL is running"
else
    echo "   ❌ PostgreSQL is not running!"
    echo "   Please start PostgreSQL first:"
    echo "   Mac: brew services start postgresql"
    echo "   Linux: sudo systemctl start postgresql"
    exit 1
fi

# Step 2: Create database
echo ""
echo "2️⃣ Creating database '${DB_NAME}'..."
PGPASSWORD=$DB_PASS psql -h $DB_HOST -U $DB_USER -c "CREATE DATABASE ${DB_NAME};" 2>/dev/null || echo "   Database already exists"

# Step 3: Install SQLx CLI if needed
echo ""
echo "3️⃣ Checking SQLx CLI..."
if ! command -v sqlx &> /dev/null; then
    echo "   Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres,rustls
else
    echo "   ✅ SQLx CLI is installed"
fi

# Step 4: Create migrations directory and file
echo ""
echo "4️⃣ Setting up migrations..."
mkdir -p migrations

# Create the migration file
cat > migrations/20240101000000_initial_schema.sql << 'EOF'
-- Initial schema for MPC Solana Wallet

-- Drop tables if they exist (for clean setup)
DROP TABLE IF EXISTS quotes CASCADE;
DROP TABLE IF EXISTS balances CASCADE;
DROP TABLE IF EXISTS assets CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    public_key VARCHAR(44),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_public_key ON users(public_key) WHERE public_key IS NOT NULL;

-- Assets table (for tokens)
CREATE TABLE assets (
    id UUID PRIMARY KEY,
    mint_address VARCHAR(44) UNIQUE NOT NULL,
    decimals INTEGER NOT NULL CHECK (decimals >= 0 AND decimals <= 18),
    name VARCHAR(255) NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    logo_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_assets_mint_address ON assets(mint_address);
CREATE INDEX idx_assets_symbol ON assets(symbol);

-- Balances table
CREATE TABLE balances (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL DEFAULT 0 CHECK (amount >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_asset UNIQUE(user_id, asset_id)
);

CREATE INDEX idx_balances_user_asset ON balances(user_id, asset_id);
CREATE INDEX idx_balances_user_id ON balances(user_id);
CREATE INDEX idx_balances_amount ON balances(amount) WHERE amount > 0;

-- Quotes table (for Jupiter swaps)
CREATE TABLE quotes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    input_mint VARCHAR(44) NOT NULL,
    output_mint VARCHAR(44) NOT NULL,
    in_amount BIGINT NOT NULL CHECK (in_amount > 0),
    out_amount BIGINT NOT NULL CHECK (out_amount > 0),
    quote_data JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_quotes_user_id ON quotes(user_id);
CREATE INDEX idx_quotes_user_expires ON quotes(user_id, expires_at);
CREATE INDEX idx_quotes_expires_at ON quotes(expires_at) WHERE used = FALSE;
CREATE INDEX idx_quotes_used ON quotes(used);

-- Insert default assets
INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url) VALUES
    ('550e8400-e29b-41d4-a716-446655440000', 'So11111111111111111111111111111111111111112', 9, 'Solana', 'SOL', 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png'),
    ('550e8400-e29b-41d4-a716-446655440001', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 6, 'USD Coin', 'USDC', 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png'),
    ('550e8400-e29b-41d4-a716-446655440002', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 6, 'Tether USD', 'USDT', 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.png')
ON CONFLICT (mint_address) DO NOTHING;
EOF

echo "   ✅ Migration file created"

# Step 5: Export DATABASE_URL and run migrations
echo ""
echo "5️⃣ Running migrations..."
export DATABASE_URL="${DATABASE_URL}"
sqlx migrate run

# Step 6: Update .env files
echo ""
echo "6️⃣ Updating .env files..."

# Root .env
cat > .env << EOF
DATABASE_URL=${DATABASE_URL}
MPC1_DATABASE_URL=${DATABASE_URL}_mpc1
MPC2_DATABASE_URL=${DATABASE_URL}_mpc2
MPC3_DATABASE_URL=${DATABASE_URL}_mpc3
JWT_SECRET=$(openssl rand -hex 32)
YELLOWSTONE_ENDPOINT=http://localhost:10000
JUPITER_API_URL=https://quote-api.jup.ag/v6
EOF

# Store .env
cat > store/.env << EOF
DATABASE_URL=${DATABASE_URL}
SQLX_OFFLINE=false
EOF

echo "   ✅ .env files created"

# Step 7: Generate sqlx-data.json for offline compilation
echo ""
echo "7️⃣ Generating SQLx offline data..."
cd store
cargo sqlx prepare --database-url "${DATABASE_URL}"
cd ..

# Step 8: Verify setup
echo ""
echo "8️⃣ Verifying database setup..."
PGPASSWORD=$DB_PASS psql -h $DB_HOST -U $DB_USER -d $DB_NAME -c "\dt" | head -10

# Step 9: Test the store module
echo ""
echo "9️⃣ Testing store module..."
cd store
cargo build
echo "   ✅ Store module builds successfully"

# Optional: Run test
read -p "Do you want to run the store test? (y/n): " run_test
if [[ $run_test == "y" ]]; then
    export TEST_DATABASE_URL="${DATABASE_URL}"
    cargo run --bin store_test
fi

cd ..

echo ""
echo "🎉 Database setup complete!"
echo ""
echo "✅ Database '${DB_NAME}' is ready with all tables"
echo "✅ Default assets (SOL, USDC, USDT) have been added"
echo "✅ SQLx offline data has been generated"
echo "✅ Store module compiles successfully"
echo ""
echo "📝 Next steps:"
echo "1. Your database is ready for use"
echo "2. You can now run: cargo run (in any module)"
echo "3. The store module will work with actual database operations"
echo ""
echo "🔑 Connection string saved in .env files:"
echo "   ${DATABASE_URL}"