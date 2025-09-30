#!/bin/bash

echo "🔄 Running Phase 4 Database Migrations"
echo "======================================"

# Database connection details
DB_USER="newuser"
DB_PASS="new_secure_password"
DB_HOST="localhost"
DB_PORT="5432"
DB_NAME="solana_wallet"

# Run the migration
echo "Running migration 002_add_balance_tables.sql..."
PGPASSWORD=$DB_PASS psql -U $DB_USER -h $DB_HOST -p $DB_PORT -d $DB_NAME -f migrations/002_add_balance_tables.sql

if [ $? -eq 0 ]; then
    echo "✅ Migration completed successfully"
else
    echo "❌ Migration failed"
    exit 1
fi

# Verify tables exist
echo ""
echo "Verifying database schema..."
PGPASSWORD=$DB_PASS psql -U $DB_USER -h $DB_HOST -p $DB_PORT -d $DB_NAME -c "
SELECT 
    table_name 
FROM information_schema.tables 
WHERE table_schema = 'public' 
ORDER BY table_name;
"

echo ""
echo "Verifying assets table..."
PGPASSWORD=$DB_PASS psql -U $DB_USER -h $DB_HOST -p $DB_PORT -d $DB_NAME -c "
SELECT id, symbol, name, decimals, mint_address FROM assets ORDER BY symbol;
"

echo ""
echo "✅ Phase 4 Database Setup Complete"
