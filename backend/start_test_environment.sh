#!/bin/bash

echo "Starting Phase 3 Test Environment..."
echo ""

# Check PostgreSQL
if ! psql -U postgres -d solana_wallet -c "SELECT 1" > /dev/null 2>&1; then
    echo "Error: PostgreSQL database not accessible"
    echo "Please ensure PostgreSQL is running and database 'solana_wallet' exists"
    exit 1
fi
echo "✓ PostgreSQL database ready"

# Start backend in background
echo "Starting backend server..."
cd backend
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"
export JWT_SECRET="your-secret-key-change-in-production-min-32-chars-long"

cargo run > ../backend.log 2>&1 &
BACKEND_PID=$!
echo $BACKEND_PID > ../backend.pid
cd ..

# Wait for backend to start
echo "Waiting for backend to start..."
sleep 3

if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✓ Backend server started (PID: $BACKEND_PID)"
else
    echo "✗ Backend failed to start. Check backend.log for details"
    exit 1
fi

echo ""
echo "Test environment ready!"
echo "- Backend: http://localhost:8080"
echo "- Logs: backend.log"
echo ""
echo "Run tests with: cd backend && ./test_phase3_complete.sh"
echo "Stop environment with: ./stop_test_environment.sh"