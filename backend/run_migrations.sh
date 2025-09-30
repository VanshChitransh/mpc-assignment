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
