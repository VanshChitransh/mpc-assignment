#!/bin/bash

# Database setup script for Solana Wallet Backend

set -e

echo "Setting up Solana Wallet Backend Database..."

# Check if .env file exists
if [ ! -f .env ]; then
    echo "Creating .env file from .env.example..."
    cp .env.example .env
    echo "⚠️  Please edit .env file with your actual database credentials and configuration!"
    echo "⚠️  Make sure PostgreSQL is running and the database exists."
fi

# Load environment variables
source .env

# Install sqlx-cli if not present
if ! command -v sqlx &> /dev/null; then
    echo "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features rustls,postgres
fi

echo "Checking database connection..."
sqlx database create 2>/dev/null || echo "Database already exists or created successfully"

echo "Running database migrations..."
sqlx migrate run --source migrations

echo "✅ Database setup completed successfully!"
echo ""
echo "Next steps:"
echo "1. Make sure all environment variables in .env are correctly configured"
echo "2. Run the backend server: cargo run --bin backend"
echo "3. Run MPC nodes: cargo run --bin mpc"
echo "4. Run indexer: cargo run --bin indexer"
echo ""
echo "For development, you might want to create a test database:"
echo "createdb solana_wallet_test"
echo "TEST_DATABASE_URL=postgresql://vansh:vansh_password@localhost:5432/solana_wallet_test sqlx migrate run --source migrations"