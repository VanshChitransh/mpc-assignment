#!/bin/bash
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

API_BASE="http://localhost:8080"
PASSED=0
FAILED=0

run_test() {
    local test_name="$1"
    local expected_status="$2"
    local actual_status="$3"
    
    if [ "$expected_status" = "$actual_status" ]; then
        echo -e "${GREEN}✅ PASS: $test_name${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}❌ FAIL: $test_name (Expected: $expected_status, Got: $actual_status)${NC}"
        FAILED=$((FAILED + 1))
    fi
}

echo -e "${BLUE}Step 3.3: Authentication Middleware Tests${NC}"
echo ""

# Test 1: Health check (public)
HEALTH_RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" "$API_BASE/health")
HEALTH_STATUS=$(echo $HEALTH_RESPONSE | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
run_test "Health check without auth" "200" "$HEALTH_STATUS"

# Test 2: Signup (public)
RANDOM_EMAIL="test_$(date +%s)@example.com"
SIGNUP_RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" -X POST "$API_BASE/api/user/signup" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$RANDOM_EMAIL\",\"password\":\"password123\"}")
SIGNUP_STATUS=$(echo $SIGNUP_RESPONSE | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
SIGNUP_BODY=$(echo $SIGNUP_RESPONSE | sed 's/HTTPSTATUS:[0-9]*$//')
run_test "User signup without auth" "201" "$SIGNUP_STATUS"

JWT_TOKEN=$(echo $SIGNUP_BODY | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
echo "Token: ${JWT_TOKEN:0:50}..."

# Test 3: Profile without auth (should fail)
PROFILE_UNAUTH=$(curl -s -w "HTTPSTATUS:%{http_code}" "$API_BASE/api/user/profile")
PROFILE_UNAUTH_STATUS=$(echo $PROFILE_UNAUTH | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
run_test "Profile without auth (should fail)" "401" "$PROFILE_UNAUTH_STATUS"

# Test 4: Profile with valid auth (should succeed)
PROFILE_AUTH=$(curl -s -w "HTTPSTATUS:%{http_code}" "$API_BASE/api/user/profile" \
  -H "Authorization: Bearer $JWT_TOKEN")
PROFILE_AUTH_STATUS=$(echo $PROFILE_AUTH | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
run_test "Profile with valid auth" "200" "$PROFILE_AUTH_STATUS"

# Test 5: Invalid token format
INVALID_FORMAT=$(curl -s -w "HTTPSTATUS:%{http_code}" "$API_BASE/api/user/profile" \
  -H "Authorization: $JWT_TOKEN")
INVALID_FORMAT_STATUS=$(echo $INVALID_FORMAT | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
run_test "Token without Bearer prefix (should fail)" "401" "$INVALID_FORMAT_STATUS"

echo -e "\n${BLUE}Test Summary${NC}"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed${NC}"
    exit 1
fi