#!/bin/bash

set -e

echo "Running all database migrations..."

# Source environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "Error: DATABASE_URL environment variable is not set"
    echo "Please create a .env file with DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_wallet"
    exit 1
fi

# Run migrations in order
echo "Migration 001: Initial schema..."
psql $DATABASE_URL -f migrations/001_initial_schema.sql

echo "Migration 002: Balance tables..."
psql $DATABASE_URL -f migrations/002_add_balance_tables.sql

echo "Migration 003: Wallet state management..."
psql $DATABASE_URL -f migrations/003_wallet_state_management.sql

echo "All migrations completed successfully!"

# Verify tables exist
echo "Verifying tables..."
psql $DATABASE_URL -c "\dt"
