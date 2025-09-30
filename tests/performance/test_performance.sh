#!/bin/bash
# Test 5: Performance & Load Tests

echo "⚡ Testing Performance & Load..."

# Test 5.1: Concurrent User Load
echo "Testing concurrent user load..."

# Create multiple users
echo "Creating test users..."
for i in {1..5}; do
  curl -s -X POST http://localhost:8080/api/user/signup \
    -H "Content-Type: application/json" \
    -d "{\"username\": \"perfuser$i\", \"password\": \"perfpass$i\"}" > /dev/null
done

# Run concurrent operations
echo "Running concurrent operations..."
RESPONSE_LOG="/tmp/api_responses.log"
> $RESPONSE_LOG

for i in {1..5}; do
  (
    TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
      -H "Content-Type: application/json" \
      -d "{\"username\": \"perfuser$i\", \"password\": \"perfpass$i\"}" | jq -r '.token')
    
    if [ "$TOKEN" != "null" ] && [ -n "$TOKEN" ]; then
      # Run multiple operations per user
      for j in {1..3}; do
        RESPONSE=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/wallet/keygen \
          -H "Authorization: Bearer $TOKEN" \
          -H "Content-Type: application/json" \
          -d '{"threshold": 2, "participants": 3}' \
          -o /dev/null)
        
        HTTP_CODE="${RESPONSE: -3}"
        echo "$HTTP_CODE" >> $RESPONSE_LOG
      done
    fi
  ) &
done

wait

# Check success rate
SUCCESS_COUNT=$(grep -c "200" $RESPONSE_LOG)
TOTAL_COUNT=$(wc -l < $RESPONSE_LOG)
SUCCESS_RATE=$((SUCCESS_COUNT * 100 / TOTAL_COUNT))

if [ $SUCCESS_RATE -ge 95 ]; then
  echo "✅ Load test passed: $SUCCESS_RATE% success rate ($SUCCESS_COUNT/$TOTAL_COUNT)"
else
  echo "❌ Load test failed: $SUCCESS_RATE% success rate ($SUCCESS_COUNT/$TOTAL_COUNT)"
fi

# Test 5.2: Latency Thresholds
echo "Testing latency thresholds..."

# Get a fresh token for latency testing
TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "testpass"}' | jq -r '.token')

if [ "$TOKEN" != "null" ] && [ -n "$TOKEN" ]; then
  # Test response time
  RESPONSE_TIME=$(curl -w "%{time_total}" -o /dev/null -s \
    -X POST http://localhost:8080/api/v1/wallet/keygen \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"threshold": 2, "participants": 3}')
  
  # Check if response time is under 5 seconds
  if (( $(echo "$RESPONSE_TIME < 5.0" | bc -l) )); then
    echo "✅ Latency test passed: ${RESPONSE_TIME}s"
  else
    echo "❌ Latency test failed: ${RESPONSE_TIME}s (threshold: 5.0s)"
  fi
else
  echo "❌ Failed to obtain token for latency testing"
fi

# Test 5.3: Memory Usage
echo "Testing memory usage..."

# Get process memory usage
BACKEND_PID=$(pgrep -f "backend")
if [ -n "$BACKEND_PID" ]; then
  MEMORY_KB=$(ps -o rss= -p $BACKEND_PID)
  MEMORY_MB=$((MEMORY_KB / 1024))
  
  if [ $MEMORY_MB -lt 500 ]; then
    echo "✅ Memory usage test passed: ${MEMORY_MB}MB (threshold: 500MB)"
  else
    echo "❌ Memory usage test failed: ${MEMORY_MB}MB (threshold: 500MB)"
  fi
else
  echo "⚠️ Backend process not found for memory testing"
fi

# Test 5.4: Database Connection Pool
echo "Testing database connection pool..."

# Test multiple concurrent database operations
for i in {1..10}; do
  curl -s -X GET http://localhost:8080/api/v1/wallet/health \
    -H "Authorization: Bearer $TOKEN" > /dev/null &
done

wait
echo "✅ Database connection pool test completed"

# Cleanup
rm -f $RESPONSE_LOG

echo "⚡ Performance & Load tests completed!"
