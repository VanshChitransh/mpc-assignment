#!/bin/bash

set -e

echo "🧪 Testing Step 3.2: User Routes with MPC Integration"
echo "====================================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

BASE_URL="http://localhost:8080"

# Test 1: Signup with MPC key generation
echo -e "\n${YELLOW}Test 1: User Signup with MPC Key Generation${NC}"
SIGNUP_RESPONSE=$(curl -s -X POST "$BASE_URL/api/user/signup" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test_step32_'$(date +%s)'@example.com",
    "password": "SecurePass123!"
  }')

echo "$SIGNUP_RESPONSE" | jq .

# Extract token and check for public_key
TOKEN=$(echo "$SIGNUP_RESPONSE" | jq -r '.token')
PUBLIC_KEY=$(echo "$SIGNUP_RESPONSE" | jq -r '.user.public_key')

if [ "$TOKEN" != "null" ] && [ "$PUBLIC_KEY" != "null" ]; then
    echo -e "${GREEN}✅ Test 1 PASSED: User created with MPC key${NC}"
else
    echo -e "${RED}❌ Test 1 FAILED: Missing token or public key${NC}"
    exit 1
fi

# Test 2: Get Profile
echo -e "\n${YELLOW}Test 2: Get User Profile${NC}"
PROFILE_RESPONSE=$(curl -s -X GET "$BASE_URL/api/user/profile" \
  -H "Authorization: Bearer $TOKEN")

echo "$PROFILE_RESPONSE" | jq .

PROFILE_PUBLIC_KEY=$(echo "$PROFILE_RESPONSE" | jq -r '.public_key')
if [ "$PROFILE_PUBLIC_KEY" = "$PUBLIC_KEY" ]; then
    echo -e "${GREEN}✅ Test 2 PASSED: Profile retrieved correctly${NC}"
else
    echo -e "${RED}❌ Test 2 FAILED: Public key mismatch${NC}"
    exit 1
fi

# Test 3: Wallet Status
echo -e "\n${YELLOW}Test 3: Check Wallet Status${NC}"
STATUS_RESPONSE=$(curl -s -X GET "$BASE_URL/api/user/wallet-status" \
  -H "Authorization: Bearer $TOKEN")

echo "$STATUS_RESPONSE" | jq .

CAN_SIGN=$(echo "$STATUS_RESPONSE" | jq -r '.mpc_health.can_sign')
if [ "$CAN_SIGN" = "true" ]; then
    echo -e "${GREEN}✅ Test 3 PASSED: MPC cluster healthy and can sign${NC}"
else
    echo -e "${YELLOW}⚠️  Test 3 WARNING: MPC cluster cannot sign (need 2/3 nodes)${NC}"
fi

# Test 4: Sign in with created user
echo -e "\n${YELLOW}Test 4: User Sign In${NC}"
USER_EMAIL=$(echo "$SIGNUP_RESPONSE" | jq -r '.user.email')
SIGNIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/user/signin" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$USER_EMAIL\",
    \"password\": \"SecurePass123!\"
  }")

echo "$SIGNIN_RESPONSE" | jq .

SIGNIN_TOKEN=$(echo "$SIGNIN_RESPONSE" | jq -r '.token')
if [ "$SIGNIN_TOKEN" != "null" ]; then
    echo -e "${GREEN}✅ Test 4 PASSED: Sign in successful${NC}"
else
    echo -e "${RED}❌ Test 4 FAILED: Sign in failed${NC}"
    exit 1
fi

# Test 5: Invalid credentials
echo -e "\n${YELLOW}Test 5: Invalid Credentials${NC}"
INVALID_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/user/signin" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "nonexistent@example.com",
    "password": "wrongpassword"
  }')

HTTP_CODE=$(echo "$INVALID_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "401" ]; then
    echo -e "${GREEN}✅ Test 5 PASSED: Invalid credentials rejected${NC}"
else
    echo -e "${RED}❌ Test 5 FAILED: Expected 401, got $HTTP_CODE${NC}"
fi

# Test 6: Email validation
echo -e "\n${YELLOW}Test 6: Email Validation${NC}"
INVALID_EMAIL=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/user/signup" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "invalid-email",
    "password": "SecurePass123!"
  }')

HTTP_CODE=$(echo "$INVALID_EMAIL" | tail -n1)
if [ "$HTTP_CODE" = "400" ]; then
    echo -e "${GREEN}✅ Test 6 PASSED: Invalid email rejected${NC}"
else
    echo -e "${RED}❌ Test 6 FAILED: Expected 400, got $HTTP_CODE${NC}"
fi

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}All Step 3.2 tests completed!${NC}"
echo -e "${GREEN}========================================${NC}"