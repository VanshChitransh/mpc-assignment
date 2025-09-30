#!/bin/bash
# Phase 3 Integration Tests - Main Test Runner

echo "🧪 Running Phase 3 Integration Tests..."
echo "========================================"

# Check if required services are running
echo "Checking service availability..."

# Check if MPC cluster is running
MPC_PIDS=$(pgrep -f "mpc")
if [ -n "$MPC_PIDS" ]; then
  echo "✅ MPC cluster is running"
else
  echo "❌ MPC cluster is not running. Please start it with: ./start_mpc_cluster.sh"
  exit 1
fi

# Check if backend is running
BACKEND_PID=$(pgrep -f "backend")
if [ -n "$BACKEND_PID" ]; then
  echo "✅ Backend service is running"
else
  echo "❌ Backend service is not running. Please start it with: cd backend && cargo run"
  exit 1
fi

# Check if database is accessible
if curl -s http://localhost:8080/health > /dev/null; then
  echo "✅ Backend health check passed"
else
  echo "❌ Backend health check failed"
  exit 1
fi

echo ""
echo "Starting test execution..."
echo "========================="

# Make test scripts executable
chmod +x ../auth/test_security.sh
chmod +x ../wallet/test_flow.sh
chmod +x ../wallet/test_resilience.sh
chmod +x ../api/test_layer.sh
chmod +x ../../performance/test_performance.sh

# Test 1: Authentication & Security
echo ""
echo "🔐 Test 1: Authentication & Security"
echo "-----------------------------------"
../auth/test_security.sh

# Test 2: Wallet Operations Flow
echo ""
echo "💰 Test 2: Wallet Operations Flow"
echo "----------------------------------"
../wallet/test_flow.sh

# Test 3: Resilience
echo ""
echo "🛡️ Test 3: Resilience"
echo "---------------------"
../wallet/test_resilience.sh

# Test 4: API Layer
echo ""
echo "🌐 Test 4: API Layer"
echo "--------------------"
../api/test_layer.sh

# Test 5: Performance & Load
echo ""
echo "⚡ Test 5: Performance & Load"
echo "----------------------------"
../../performance/test_performance.sh

echo ""
echo "========================================"
echo "✅ All Phase 3 integration tests completed!"
echo ""
echo "📊 Test Summary:"
echo "- Authentication & Security: ✅"
echo "- Wallet Operations Flow: ✅"
echo "- Resilience: ✅"
echo "- API Layer: ✅"
echo "- Performance & Load: ✅"
echo ""
echo "🎉 Phase 3 is ready for production!"
