#!/bin/bash
# Test 2: Wallet Operations Flow Tests

echo "�� Testing Wallet Operations Flow..."

# Get authentication token
TOKEN=$(curl -s -X POST http://localhost:8080/api/user/signin \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "testpass"}' | jq -r '.token')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Failed to obtain authentication token"
  exit 1
fi

# Test 2.1: Complete Signing Flow
echo "Testing complete signing flow..."

# Step 1: Generate keys
echo "Step 1: Generating keys..."
KEYGEN_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}')

PUBLIC_KEY=$(echo $KEYGEN_RESPONSE | jq -r '.data.public_key')
if [ "$PUBLIC_KEY" != "null" ] && [ -n "$PUBLIC_KEY" ]; then
  echo "✅ Key generation successful: $PUBLIC_KEY"
else
  echo "❌ Key generation failed"
  exit 1
fi

# Step 2: Sign Phase 1
echo "Step 2: Sign Phase 1..."
PHASE1_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/wallet/sign/phase1 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello, MPC Wallet!"}')

SESSION_ID=$(echo $PHASE1_RESPONSE | jq -r '.data.session_id')
NONCE_COMMITMENT=$(echo $PHASE1_RESPONSE | jq -r '.data.nonce_commitment')

if [ "$SESSION_ID" != "null" ] && [ -n "$SESSION_ID" ]; then
  echo "✅ Sign Phase 1 successful: Session $SESSION_ID"
else
  echo "❌ Sign Phase 1 failed"
  exit 1
fi

# Step 3: Sign Phase 2
echo "Step 3: Sign Phase 2..."
PHASE2_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/wallet/sign/phase2 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"session_id": "'$SESSION_ID'", "message": "Hello, MPC Wallet!"}')

SIGNATURE_SHARE=$(echo $PHASE2_RESPONSE | jq -r '.data.signature_share')

if [ "$SIGNATURE_SHARE" != "null" ] && [ -n "$SIGNATURE_SHARE" ]; then
  echo "✅ Sign Phase 2 successful: Signature share generated"
else
  echo "❌ Sign Phase 2 failed"
  exit 1
fi

# Step 4: Aggregate
echo "Step 4: Aggregating signatures..."
AGGREGATE_RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/wallet/aggregate \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"session_id": "'$SESSION_ID'"}')

FINAL_SIGNATURE=$(echo $AGGREGATE_RESPONSE | jq -r '.data.signature')

if [ "$FINAL_SIGNATURE" != "null" ] && [ -n "$FINAL_SIGNATURE" ]; then
  echo "✅ Signature aggregation successful: $FINAL_SIGNATURE"
else
  echo "❌ Signature aggregation failed"
  exit 1
fi

# Test 2.2: Idempotency Validation
echo "Testing idempotency..."

KEYGEN1=$(curl -s -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}')

KEYGEN2=$(curl -s -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}')

KEY1=$(echo $KEYGEN1 | jq -r '.data.public_key')
KEY2=$(echo $KEYGEN2 | jq -r '.data.public_key')

if [ "$KEY1" = "$KEY2" ]; then
  echo "✅ Idempotency test passed: Same key returned"
else
  echo "❌ Idempotency test failed: Different keys returned"
fi

echo "💰 Wallet Operations Flow tests completed!"
