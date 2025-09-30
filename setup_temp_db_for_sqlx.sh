#!/bin/bash

set -e

echo "Setting up temporary database for sqlx query cache generation..."

# Create a temporary database URL
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet_temp"

# Check if PostgreSQL is running
if ! pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
    echo "PostgreSQL is not running. Starting it..."
    if command -v brew >/dev/null 2>&1; then
        brew services start postgresql
        sleep 5
    else
        echo "Please start PostgreSQL manually"
        exit 1
    fi
fi

# Create temporary database
echo "Creating temporary database..."
createdb solana_wallet_temp 2>/dev/null || echo "Database might already exist"

# Create postgres user if it doesn't exist
psql -d solana_wallet_temp -c "CREATE USER postgres WITH PASSWORD 'postgres';" 2>/dev/null || echo "User might already exist"
psql -d solana_wallet_temp -c "GRANT ALL PRIVILEGES ON DATABASE solana_wallet_temp TO postgres;" 2>/dev/null || echo "Privileges might already be granted"

# Run migrations on temporary database
echo "Running migrations on temporary database..."
psql $DATABASE_URL -f migrations/001_initial_schema.sql
psql $DATABASE_URL -f migrations/002_add_balance_tables.sql  
psql $DATABASE_URL -f migrations/003_wallet_state_management.sql

echo "Temporary database setup complete!"
echo "DATABASE_URL is set to: $DATABASE_URL"
