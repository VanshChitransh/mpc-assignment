#!/bin/bash

# Fix PostgreSQL permissions for MPC Solana Wallet project

echo "🔧 Fixing PostgreSQL permissions and database setup..."

# First, let's check the current user and database status
echo "📊 Current database status:"
psql -U postgres -c "\l" 2>/dev/null || echo "⚠️  Cannot connect as postgres user"

# Option 1: Connect as superuser and fix permissions
echo "🔐 Attempting to fix permissions as superuser..."

# Connect as your local user (which is likely a superuser)
psql -d solana_wallet << 'EOF'
-- Fix ownership of database and objects
ALTER DATABASE solana_wallet OWNER TO postgres;

-- Fix table ownership
ALTER TABLE IF EXISTS users OWNER TO postgres;
ALTER TABLE IF EXISTS balances OWNER TO postgres;
ALTER TABLE IF EXISTS assets OWNER TO postgres;
ALTER TABLE IF EXISTS quotes OWNER TO postgres;
ALTER TABLE IF EXISTS wallet_keys OWNER TO postgres;
ALTER TABLE IF EXISTS signing_sessions OWNER TO postgres;

-- Grant all privileges to postgres user
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO postgres;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO postgres;
GRANT USAGE ON SCHEMA public TO postgres;

-- Set default privileges for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO postgres;

-- Show current ownership
\dt
EOF

echo "✅ Fixed database ownership"

# Option 2: Alternative - Recreate the database cleanly
echo "🔄 Alternative: Clean database recreation..."

read -p "Do you want to recreate the database cleanly? This will delete all data (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Drop and recreate database
    psql -U postgres << 'EOF'
DROP DATABASE IF EXISTS solana_wallet;
CREATE DATABASE solana_wallet OWNER postgres;
EOF

    # Apply our migration
    export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"
    cd backend
    cargo sqlx migrate run

    echo "✅ Database recreated successfully"
fi

# Add the missing columns we need
echo "📝 Adding missing database columns..."
psql -U postgres -d solana_wallet << 'EOF'
-- Add last_updated column to balances if it doesn't exist
ALTER TABLE balances ADD COLUMN IF NOT EXISTS last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Create index if it doesn't exist
CREATE INDEX IF NOT EXISTS idx_balances_last_updated ON balances(last_updated);

-- Add any other missing columns
ALTER TABLE balances ADD COLUMN IF NOT EXISTS confirmed BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE quotes ADD COLUMN IF NOT EXISTS used_at TIMESTAMPTZ;

-- Show final table structure
\d balances
\d assets
\d users
\d quotes
EOF

echo "🎉 Database setup completed!"

# Test the connection
echo "🧪 Testing database connection..."
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"
psql "${DATABASE_URL}" -c "SELECT 'Connection successful!' as status;"

echo "✅ All done! You can now run 'cargo build' in the backend directory."