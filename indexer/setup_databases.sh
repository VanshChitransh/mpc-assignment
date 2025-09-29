#!/bin/bash

echo "🗄️ Setting up database schema for Solana Wallet Indexer"
echo "======================================================="

# Check if PostgreSQL is running
if ! docker ps | grep -q postgres; then
    echo "[ERROR] PostgreSQL container not running. Please run ./setup.sh first."
    exit 1
fi

# Create migrations directory if it doesn't exist
mkdir -p migrations

# Check if migration file exists
if [ ! -f "migrations/001_initial.sql" ]; then
    echo "[INFO] Creating initial migration file..."
    cat > migrations/001_initial.sql << 'EOF'
-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    sol_balance BIGINT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User wallets table
CREATE TABLE user_wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    address VARCHAR(44) UNIQUE NOT NULL, -- Solana addresses are base58, max 44 chars
    sol_balance BIGINT DEFAULT 0, -- Store in lamports (1 SOL = 1e9 lamports)
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Balance changes tracking
CREATE TABLE balance_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    address VARCHAR(44) NOT NULL,
    old_balance BIGINT NOT NULL,
    new_balance BIGINT NOT NULL,
    slot BIGINT NOT NULL,
    transaction_signature VARCHAR(88), -- Solana tx signatures are base58, max 88 chars
    change_type VARCHAR(50) NOT NULL, -- 'Transfer', 'Reward', 'Fee', etc.
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Token balances (for SPL tokens)
CREATE TABLE token_balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_account VARCHAR(44) NOT NULL, -- Token account address
    mint VARCHAR(44) NOT NULL, -- Token mint address
    amount BIGINT NOT NULL, -- Token amount in smallest unit
    slot BIGINT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Transactions tracking
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    signature VARCHAR(88) UNIQUE NOT NULL,
    slot BIGINT NOT NULL,
    accounts TEXT[], -- Array of account addresses involved
    pre_balances BIGINT[], -- Balances before transaction
    post_balances BIGINT[], -- Balances after transaction
    logs TEXT[], -- Transaction logs
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexer state management
CREATE TABLE indexer_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(100) UNIQUE NOT NULL,
    value TEXT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Subscription metrics
CREATE TABLE subscription_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_name VARCHAR(100) NOT NULL,
    metric_value BIGINT NOT NULL,
    tags JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_user_wallets_user_id ON user_wallets(user_id);
CREATE INDEX idx_user_wallets_address ON user_wallets(address);
CREATE INDEX idx_user_wallets_active ON user_wallets(is_active);
CREATE INDEX idx_balance_changes_address ON balance_changes(address);
CREATE INDEX idx_balance_changes_created_at ON balance_changes(created_at);
CREATE INDEX idx_balance_changes_slot ON balance_changes(slot);
CREATE INDEX idx_token_balances_token_account ON token_balances(token_account);
CREATE INDEX idx_token_balances_mint ON token_balances(mint);
CREATE INDEX idx_transactions_signature ON transactions(signature);
CREATE INDEX idx_transactions_slot ON transactions(slot);
CREATE INDEX idx_indexer_state_key ON indexer_state(key);
CREATE INDEX idx_subscription_metrics_name ON subscription_metrics(metric_name);
CREATE INDEX idx_subscription_metrics_created_at ON subscription_metrics(created_at);
EOF
    echo "[SUCCESS] Migration file created"
fi

# Run the migration
echo "[INFO] Running database migration..."
if docker exec $(docker ps --filter "ancestor=postgres:15" --format "{{.ID}}" | head -n 1) psql -U postgres -d indexer_db -f /dev/stdin < migrations/001_initial.sql; then
    echo "[SUCCESS] Database schema created successfully!"
else
    echo "[ERROR] Failed to run migration. Trying alternative method..."
    
    # Try using host psql if docker exec fails
    if command -v psql >/dev/null 2>&1; then
        PGPASSWORD=password psql -h localhost -p 5432 -U postgres -d indexer_db -f migrations/001_initial.sql
    else
        echo "[ERROR] Could not run migration. Please run manually:"
        echo "psql -h localhost -p 5432 -U postgres -d indexer_db -f migrations/001_initial.sql"
        exit 1
    fi
fi

echo ""
echo "🎉 Database setup complete!"
echo "You can now run: cargo check"