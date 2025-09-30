#!/bin/bash

# Phase 3 Authentication & Rate Limiting Test Script
# Tests: Auth middleware, Rate limiting, User signup/signin

set -e

BASE_URL="http://localhost:8080"
PASS_COUNT=0
FAIL_COUNT=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_test() {
    echo -e "${YELLOW}[TEST]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

# Generate unique test user
TIMESTAMP=$(date +%s)
TEST_USER="test_${TIMESTAMP}@example.com"
TEST_PASS="SecurePass123!"

echo "========================================="
echo "Phase 3: Authentication & Security Tests"
echo "========================================="
echo ""

# Test 1: Protected endpoint without token should fail
log_test "Test 1: Access protected endpoint without token"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/api/user/profile")
if [ "$HTTP_CODE" = "401" ]; then
    log_pass "Protected endpoint blocks unauthenticated requests (401)"
else
    log_fail "Expected 401, got $HTTP_CODE"
fi
echo ""

# Test 2: User signup
log_test "Test 2: User signup"
SIGNUP_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${BASE_URL}/api/user/signup" \
    -H "Content-Type: application/json" \
    -d "{
        \"username\": \"${TEST_USER}\",
        \"email\": \"${TEST_USER}\",
        \"password\": \"${TEST_PASS}\"
    }")

HTTP_CODE=$(echo "$SIGNUP_RESPONSE" | tail -1)
RESPONSE_BODY=$(echo "$SIGNUP_RESPONSE" | head -n -1)

if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
    log_pass "User signup successful ($HTTP_CODE)"
    echo "Response: $RESPONSE_BODY"
else
    log_fail "Signup failed with code $HTTP_CODE"
    echo "Response: $RESPONSE_BODY"
fi
echo ""

# Test 3: User signin and get token
log_test "Test 3: User signin"
SIGNIN_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${BASE_URL}/api/user/signin" \
    -H "Content-Type: application/json" \
    -d "{
        \"username\": \"${TEST_USER}\",
        \"password\": \"${TEST_PASS}\"
    }")

HTTP_CODE=$(echo "$SIGNIN_RESPONSE" | tail -1)
RESPONSE_BODY=$(echo "$SIGNIN_RESPONSE" | head -n -1)

if [ "$HTTP_CODE" = "200" ]; then
    TOKEN=$(echo "$RESPONSE_BODY" | grep -o '"token":"[^"]*' | cut -d'"' -f4)
    if [ -n "$TOKEN" ]; then
        log_pass "Signin successful, token received"
        echo "Token (first 20 chars): ${TOKEN:0:20}..."
    else
        log_fail "Signin returned 200 but no token found"
        echo "Response: $RESPONSE_BODY"
        TOKEN=""
    fi
else
    log_fail "Signin failed with code $HTTP_CODE"
    echo "Response: $RESPONSE_BODY"
    TOKEN=""
fi
echo ""

# Test 4: Access protected endpoint with valid token
if [ -n "$TOKEN" ]; then
    log_test "Test 4: Access protected endpoint with valid token"
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/api/user/profile" \
        -H "Authorization: Bearer $TOKEN")
    
    if [ "$HTTP_CODE" = "200" ]; then
        log_pass "Protected endpoint accessible with valid token (200)"
    else
        log_fail "Expected 200, got $HTTP_CODE"
    fi
else
    log_fail "Test 4: Skipped (no valid token from signin)"
fi
echo ""

# Test 5: Access protected endpoint with invalid token
log_test "Test 5: Access protected endpoint with invalid token"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/api/user/profile" \
    -H "Authorization: Bearer invalid_token_12345")

if [ "$HTTP_CODE" = "401" ]; then
    log_pass "Invalid token rejected (401)"
else
    log_fail "Expected 401, got $HTTP_CODE"
fi
echo ""

# Test 6: Rate limiting (if token is available)
if [ -n "$TOKEN" ]; then
    log_test "Test 6: Rate limiting (sending 105 requests)"
    echo "This may take a minute..."
    
    rate_limit_hit=false
    for i in {1..105}; do
        HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/api/user/profile" \
            -H "Authorization: Bearer $TOKEN")
        
        if [ "$HTTP_CODE" = "429" ]; then
            log_pass "Rate limit triggered after $i requests (429)"
            rate_limit_hit=true
            break
        fi
    done
    
    if [ "$rate_limit_hit" = false ]; then
        log_fail "Rate limit not triggered after 105 requests"
    fi
else
    log_fail "Test 6: Skipped (no valid token)"
fi
echo ""

# Summary
echo "========================================="
echo "Test Summary"
echo "========================================="
echo -e "${GREEN}Passed: $PASS_COUNT${NC}"
echo -e "${RED}Failed: $FAIL_COUNT${NC}"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}✅ All Phase 3 tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi
