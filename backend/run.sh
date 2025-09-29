#!/bin/bash

echo "🔍 Testing Solana Wallet Backend API (Fixed)"
echo "=============================================="

# 1. Health check
echo -e "\n1. Health Check:"
curl -w "\nStatus: %{http_code}\n" http://localhost:8080/health

# 2. Register with email format username
echo -e "\n2. User Registration (with email format):"
SIGNUP_RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}" -X POST http://localhost:8080/api/user/signup \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser@example.com","password":"password123"}')

echo "$SIGNUP_RESPONSE"
SIGNUP_STATUS=$(echo "$SIGNUP_RESPONSE" | grep "HTTP_STATUS" | cut -d: -f2)

if [ "$SIGNUP_STATUS" = "201" ]; then
    echo "✅ Registration successful"
else
    echo "❌ Registration failed with status: $SIGNUP_STATUS"
fi

# 3. Sign in with email format
echo -e "\n3. User Sign In:"
SIGNIN_RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}" -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username":"newuser@example.com","password":"password123"}')

echo "$SIGNIN_RESPONSE"
SIGNIN_STATUS=$(echo "$SIGNIN_RESPONSE" | grep "HTTP_STATUS" | cut -d: -f2)

# Extract JWT token
JWT_TOKEN=$(echo "$SIGNIN_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

if [ "$SIGNIN_STATUS" = "200" ] && [ ! -z "$JWT_TOKEN" ]; then
    echo "✅ Sign in successful"
    echo "🔑 JWT Token: $JWT_TOKEN"
else
    echo "❌ Sign in failed"
    exit 1
fi

# 4. Get balance
echo -e "\n4. Get Balance:"
BALANCE_RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}" http://localhost:8080/api/solana/balance \
  -H "Authorization: Bearer $JWT_TOKEN")

echo "$BALANCE_RESPONSE"
BALANCE_STATUS=$(echo "$BALANCE_RESPONSE" | grep "HTTP_STATUS" | cut -d: -f2)

if [ "$BALANCE_STATUS" = "200" ]; then
    echo "✅ Balance check successful"
else
    echo "❌ Balance check failed with status: $BALANCE_STATUS"
fi

echo -e "\n📋 Test Summary:"
echo "================="
echo "All endpoints tested with email format username!"