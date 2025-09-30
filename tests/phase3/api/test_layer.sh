#!/bin/bash
# Test 4: API Layer Tests

echo "🌐 Testing API Layer..."

# Get authentication token
TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "testpass"}' | jq -r '.token')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Failed to obtain authentication token"
  exit 1
fi

# Test 4.1: CORS Headers Validation
echo "Testing CORS headers..."

# Test CORS preflight request
RESPONSE=$(curl -s -I -X OPTIONS http://localhost:8080/api/v1/wallet/health \
  -H "Origin: https://example.com" \
  -H "Access-Control-Request-Method: GET" \
  -H "Access-Control-Request-Headers: Authorization")

if echo "$RESPONSE" | grep -q "Access-Control-Allow-Origin"; then
  echo "✅ CORS preflight test passed: Headers present"
else
  echo "❌ CORS preflight test failed: Headers missing"
fi

# Check response headers for actual request
RESPONSE=$(curl -s -I -X GET http://localhost:8080/api/v1/wallet/health \
  -H "Origin: https://example.com" \
  -H "Authorization: Bearer $TOKEN")

if echo "$RESPONSE" | grep -q "Access-Control-Allow-Origin"; then
  echo "✅ CORS headers test passed: Access-Control-Allow-Origin present"
else
  echo "❌ CORS headers test failed: Access-Control-Allow-Origin missing"
fi

# Test 4.2: Rate Limiting
echo "Testing rate limiting..."

# Count successful requests
SUCCESS_COUNT=0
RATE_LIMIT_HIT=false

for i in {1..105}; do
  RESPONSE=$(curl -s -w "%{http_code}" -X GET http://localhost:8080/api/v1/wallet/health \
    -H "Authorization: Bearer $TOKEN" \
    -o /dev/null)
  
  HTTP_CODE="${RESPONSE: -3}"
  
  if [ "$HTTP_CODE" = "200" ]; then
    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
  elif [ "$HTTP_CODE" = "429" ]; then
    RATE_LIMIT_HIT=true
    echo "Rate limit hit at request $i"
    break
  fi
done

if [ "$RATE_LIMIT_HIT" = true ]; then
  echo "✅ Rate limiting test passed: Rate limit enforced at request $((SUCCESS_COUNT + 1))"
else
  echo "❌ Rate limiting test failed: No rate limit hit after 105 requests"
fi

# Test 4.3: OpenAPI Documentation
echo "Testing OpenAPI documentation..."

# Test Swagger UI accessibility
RESPONSE=$(curl -s -I http://localhost:8080/api/docs/)
HTTP_CODE=$(echo "$RESPONSE" | head -n 1 | cut -d' ' -f2)

if [ "$HTTP_CODE" = "200" ]; then
  echo "✅ Swagger UI test passed: 200 OK"
else
  echo "❌ Swagger UI test failed: Expected 200, got $HTTP_CODE"
fi

# Test OpenAPI spec
API_TITLE=$(curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.title')

if [ "$API_TITLE" != "null" ] && [ -n "$API_TITLE" ]; then
  echo "✅ OpenAPI spec test passed: $API_TITLE"
else
  echo "❌ OpenAPI spec test failed: No title found"
fi

# Test 4.4: Standardized API Responses
echo "Testing standardized API responses..."

RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}')

# Check response structure
HAS_SUCCESS=$(echo $RESPONSE | jq -r '.success')
HAS_DATA=$(echo $RESPONSE | jq -r '.data')

if [ "$HAS_SUCCESS" = "true" ] && [ "$HAS_DATA" != "null" ]; then
  echo "✅ Standardized response test passed: Proper structure"
else
  echo "❌ Standardized response test failed: Invalid structure"
fi

# Test error response structure
ERROR_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}')

HAS_ERROR_SUCCESS=$(echo $ERROR_RESPONSE | jq -r '.success')
HAS_ERROR_FIELD=$(echo $ERROR_RESPONSE | jq -r '.error')

if [ "$HAS_ERROR_SUCCESS" = "false" ] && [ "$HAS_ERROR_FIELD" != "null" ]; then
  echo "✅ Error response test passed: Proper error structure"
else
  echo "❌ Error response test failed: Invalid error structure"
fi

echo "🌐 API Layer tests completed!"
