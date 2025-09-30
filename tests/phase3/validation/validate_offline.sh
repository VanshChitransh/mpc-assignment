#!/bin/bash

set -e

echo "=== Phase 3 Validation Script (Offline Mode) ==="

# Check 1: Database tables exist (skip if no database)
echo "1. Checking database tables..."
if [ -n "$DATABASE_URL" ] && command -v psql >/dev/null 2>&1; then
    psql $DATABASE_URL -c "\dt" | grep -q "wallet_keys" && echo "✓ wallet_keys table exists" || echo "✗ wallet_keys table missing"
    psql $DATABASE_URL -c "\dt" | grep -q "signing_sessions" && echo "✓ signing_sessions table exists" || echo "✗ signing_sessions table missing"
else
    echo "⚠ DATABASE_URL not set or psql not available, skipping database checks"
fi

# Check 2: Backend compiles (offline mode)
echo "2. Checking backend compilation (offline mode)..."
cd backend
SQLX_OFFLINE=true cargo check 2>&1 | tee /tmp/backend_check.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✓ Backend compiles successfully"
else
    echo "✗ Backend has compilation errors"
    cat /tmp/backend_check.log
    exit 1
fi
cd ..

# Check 3: MPC nodes compile
echo "3. Checking MPC compilation..."
cd mpc
cargo check 2>&1 | tee /tmp/mpc_check.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✓ MPC compiles successfully"
else
    echo "✗ MPC has compilation errors"
    cat /tmp/mpc_check.log
    exit 1
fi
cd ..

# Check 4: Store module compiles (offline mode)
echo "4. Checking store compilation (offline mode)..."
cd store
SQLX_OFFLINE=true cargo check 2>&1 | tee /tmp/store_check.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✓ Store compiles successfully"
else
    echo "✗ Store has compilation errors"
    cat /tmp/store_check.log
    exit 1
fi
cd ..

echo ""
echo "=== Validation Complete ==="
echo "Phase 3 is ready for testing if all checks passed."
echo ""
echo "Next steps:"
echo "1. Set up PostgreSQL database"
echo "2. Run migrations: ./run_all_migrations.sh"
echo "3. Start MPC cluster: ./start_mpc_cluster.sh"
echo "4. Start backend: cd backend && cargo run"
