#!/bin/bash
# Test 1: Authentication & Security Tests

echo "🔐 Testing Authentication & Security..."

# Test 1.1: JWT Authentication Validation
echo "Testing JWT authentication..."

# Test: Requests without JWT should fail (401)
echo "Testing request without JWT..."
RESPONSE=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}')

HTTP_CODE="${RESPONSE: -3}"
if [ "$HTTP_CODE" = "401" ]; then
  echo "✅ JWT authentication test passed: 401 Unauthorized"
else
  echo "❌ JWT authentication test failed: Expected 401, got $HTTP_CODE"
fi

# Test: Valid JWT should succeed
echo "Testing valid JWT authentication..."
TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "testpass"}' | jq -r '.token')

if [ "$TOKEN" != "null" ] && [ -n "$TOKEN" ]; then
  RESPONSE=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/wallet/keygen \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"threshold": 2, "participants": 3}')
  
  HTTP_CODE="${RESPONSE: -3}"
  if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ Valid JWT test passed: 200 OK"
  else
    echo "❌ Valid JWT test failed: Expected 200, got $HTTP_CODE"
  fi
else
  echo "❌ Failed to obtain JWT token"
fi

# Test 1.2: User Isolation
echo "Testing user isolation..."

# Create two users
USER1_TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username": "user1", "password": "pass1"}' | jq -r '.token')

USER2_TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username": "user2", "password": "pass2"}' | jq -r '.token')

if [ "$USER1_TOKEN" != "null" ] && [ "$USER2_TOKEN" != "null" ]; then
  # User1 creates session
  SESSION_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/wallet/sign/phase1 \
    -H "Authorization: Bearer $USER1_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"message": "test message"}')
  
  SESSION_ID=$(echo $SESSION_RESPONSE | jq -r '.data.session_id')
  
  if [ "$SESSION_ID" != "null" ] && [ -n "$SESSION_ID" ]; then
    # User2 tries to access User1's session
    RESPONSE=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/wallet/sign/phase2 \
      -H "Authorization: Bearer $USER2_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"session_id": "'$SESSION_ID'", "message": "test message"}')
    
    HTTP_CODE="${RESPONSE: -3}"
    if [ "$HTTP_CODE" = "400" ]; then
      echo "✅ User isolation test passed: 400 Bad Request"
    else
      echo "❌ User isolation test failed: Expected 400, got $HTTP_CODE"
    fi
  else
    echo "❌ Failed to create session for user isolation test"
  fi
else
  echo "❌ Failed to create users for isolation test"
fi

echo "🔐 Authentication & Security tests completed!"
