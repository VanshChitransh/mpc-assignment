#!/bin/bash
# Test 3: Resilience Tests

echo "🛡️ Testing Resilience..."

# Get authentication token
TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "testpass"}' | jq -r '.token')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Failed to obtain authentication token"
  exit 1
fi

# Test 3.1: Single Node Failure
echo "Testing single node failure resilience..."

# Check if MPC processes are running
MPC_PIDS=$(pgrep -f "mpc.*node1")
if [ -n "$MPC_PIDS" ]; then
  echo "Stopping one MPC node (node1)..."
  pkill -f "mpc.*node1"
  sleep 2
  
  # Test signing should still succeed (2/3 nodes available)
  RESPONSE=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/wallet/sign/phase1 \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"message": "test with 2 nodes"}')
  
  HTTP_CODE="${RESPONSE: -3}"
  if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ Single node failure test passed: 200 OK with 2/3 nodes"
  else
    echo "❌ Single node failure test failed: Expected 200, got $HTTP_CODE"
  fi
  
  # Restart the node
  echo "Restarting MPC node1..."
  ./start_mpc_cluster.sh &
  sleep 5
else
  echo "⚠️ MPC node1 not found, skipping single node failure test"
fi

# Test 3.2: Multiple Node Failure
echo "Testing multiple node failure resilience..."

# Stop two MPC nodes
echo "Stopping two MPC nodes..."
pkill -f "mpc.*node1"
pkill -f "mpc.*node2"
sleep 2

# Test signing should fail gracefully
RESPONSE=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/wallet/sign/phase1 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message": "test with 1 node"}')

HTTP_CODE="${RESPONSE: -3}"
if [ "$HTTP_CODE" = "503" ]; then
  echo "✅ Multiple node failure test passed: 503 Service Unavailable"
else
  echo "❌ Multiple node failure test failed: Expected 503, got $HTTP_CODE"
fi

# Restart the cluster
echo "Restarting MPC cluster..."
./start_mpc_cluster.sh &
sleep 10

# Test 3.3: Retry Logic Validation
echo "Testing retry logic..."

# Simulate temporary network issues
echo "Simulating temporary network issues..."
pkill -f "mpc.*node1"
sleep 3
./start_mpc_cluster.sh &
sleep 5

# Test should succeed after retry
RESPONSE=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}')

HTTP_CODE="${RESPONSE: -3}"
if [ "$HTTP_CODE" = "200" ]; then
  echo "✅ Retry logic test passed: 200 OK after retry"
else
  echo "❌ Retry logic test failed: Expected 200, got $HTTP_CODE"
fi

echo "🛡️ Resilience tests completed!"
