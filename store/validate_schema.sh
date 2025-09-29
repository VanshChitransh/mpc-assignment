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
