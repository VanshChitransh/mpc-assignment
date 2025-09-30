#!/bin/bash

set -e

BASE_URL="http://localhost:8080"
TEST_EMAIL="test_$(date +%s)@example.com"
TEST_PASSWORD="SecurePass123!"

echo "==================================="
echo "Phase 3 Complete Integration Tests"
echo "==================================="
echo ""

declare -i PASS_COUNT=0
declare -i FAIL_COUNT=0

run_test() {
    local test_name="$1"
    local expected_status="$2"
    shift 2
    local response
    
    echo -n "Testing: $test_name ... "
    
    response=$(curl -s -w "\n%{http_code}" "$@")
    status_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$status_code" = "$expected_status" ]; then
        echo "✓ PASS"
        ((PASS_COUNT++))
        return 0
    else
        echo "✗ FAIL (expected $expected_status, got $status_code)"
        echo "  Response: $body"
        ((FAIL_COUNT++))
        return 1
    fi
}

# Test 1: Health check
echo "=== Test 1: Health Check ==="
run_test "Health endpoint" "200" -X GET "$BASE_URL/health"
echo ""

# Test 2: Signup
echo "=== Test 2: User Signup ==="
SIGNUP_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/user/signup" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$TEST_EMAIL\",\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}")

SIGNUP_STATUS=$(echo "$SIGNUP_RESPONSE" | tail -n 1)
SIGNUP_BODY=$(echo "$SIGNUP_RESPONSE" | sed '$d')

if [ "$SIGNUP_STATUS" = "201" ] || [ "$SIGNUP_STATUS" = "200" ]; then
    echo "✓ PASS: Signup successful"
    ((PASS_COUNT++))
else
    echo "✗ FAIL: Signup failed with status $SIGNUP_STATUS"
    ((FAIL_COUNT++))
fi
echo ""

# Test 3: Signin
echo "=== Test 3: User Signin ==="
SIGNIN_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/user/signin" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}")

SIGNIN_STATUS=$(echo "$SIGNIN_RESPONSE" | tail -n 1)
SIGNIN_BODY=$(echo "$SIGNIN_RESPONSE" | sed '$d')

if [ "$SIGNIN_STATUS" = "200" ]; then
    echo "✓ PASS: Signin successful"
    ((PASS_COUNT++))
    TOKEN=$(echo "$SIGNIN_BODY" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
    echo "  Token: ${TOKEN:0:30}..."
else
    echo "✗ FAIL: Signin failed"
    ((FAIL_COUNT++))
    exit 1
fi
echo ""

# Test 4: Protected endpoint without token
echo "=== Test 4: Auth Required ==="
run_test "Profile without token" "401" -X GET "$BASE_URL/api/user/profile"
echo ""

# Test 5: Protected endpoint with token
echo "=== Test 5: Valid Auth ==="
run_test "Profile with token" "200" \
    -X GET "$BASE_URL/api/user/profile" \
    -H "Authorization: Bearer $TOKEN"
echo ""

# Test 6: Invalid token
echo "=== Test 6: Invalid Auth ==="
run_test "Invalid token" "401" \
    -X GET "$BASE_URL/api/user/profile" \
    -H "Authorization: Bearer invalid_token"
echo ""

# Test 7: Rate limiting
echo "=== Test 7: Rate Limiting ==="
echo "Rapidly sending requests to trigger rate limit..."

RATE_LIMIT_HIT=false
for i in {1..110}; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X GET "$BASE_URL/api/user/profile" \
        -H "Authorization: Bearer $TOKEN" 2>/dev/null || echo "000")
    
    if [ "$STATUS" = "429" ]; then
        RATE_LIMIT_HIT=true
        echo "✓ PASS: Rate limit triggered at request #$i"
        ((PASS_COUNT++))
        break
    fi
    
    # Show progress every 20 requests
    if [ $((i % 20)) -eq 0 ]; then
        echo "  ... sent $i requests (status: $STATUS)"
    fi
done

if [ "$RATE_LIMIT_HIT" = false ]; then
    echo "✗ FAIL: Rate limit NOT triggered after 110 requests"
    echo "  Note: Rate limiting by IP may not work on localhost"
    echo "  Middleware is installed but may need production testing"
    ((FAIL_COUNT++))
fi
echo ""

# Summary
echo "==================================="
echo "         TEST SUMMARY"
echo "==================================="
echo "✓ PASSED: $PASS_COUNT"
echo "✗ FAILED: $FAIL_COUNT"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo "🎉 All tests PASSED!"
    echo ""
    echo "Phase 3 Status: ✓ COMPLETE"
    exit 0
else
    echo "⚠️  Some tests failed"
    echo ""
    if [ $PASS_COUNT -ge 6 ]; then
        echo "Phase 3 Status: MOSTLY COMPLETE (core functionality working)"
        echo "Note: Rate limiting may work in production but not localhost testing"
    fi
    exit 1
fi