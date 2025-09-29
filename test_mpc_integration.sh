#!/bin/bash

echo "🧪 Testing MPC Integration"
echo "========================="

USER_ID="550e8400-e29b-41d4-a716-446655440000"
TEST_MESSAGE="Hello MPC World"
MESSAGE_HASH=$(echo -n "$TEST_MESSAGE" | shasum -a 256 | cut -d' ' -f1)

echo "Test User ID: $USER_ID"
echo "Test Message: $TEST_MESSAGE"
echo "Message Hash: $MESSAGE_HASH"
echo ""

echo "1. Testing Key Generation..."
echo "----------------------------"

KEY_GEN_PAYLOAD=$(cat <<EOF
{
    "user_id": "$USER_ID",
    "threshold": 2,
    "total_parties": 3
}
EOF
)

echo "Sending key generation request to all nodes:"
for port in 8001 8002 8003; do
    echo -n "Node $port: "
    RESPONSE=$(curl -s -X POST "http://localhost:$port/generate" \
        -H "Content-Type: application/json" \
        -d "$KEY_GEN_PAYLOAD")
    
    if command -v jq >/dev/null 2>&1; then
        if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
            echo "✅ SUCCESS"
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    else
        if [[ "$RESPONSE" == *"\"success\":true"* ]]; then
            echo "✅ SUCCESS"
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    fi
done

sleep 1

echo ""
echo "2. Testing Public Key Aggregation..."
echo "------------------------------------"

AGGREGATE_PAYLOAD=$(cat <<EOF
{
    "user_id": "$USER_ID"
}
EOF
)

for port in 8001 8002 8003; do
    echo -n "Node $port: "
    RESPONSE=$(curl -s -X POST "http://localhost:$port/aggregate-keys" \
        -H "Content-Type: application/json" \
        -d "$AGGREGATE_PAYLOAD")
    
    if command -v jq >/dev/null 2>&1; then
        if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
            PUBLIC_KEY=$(echo "$RESPONSE" | jq -r '.public_key')
            echo "✅ SUCCESS - Public Key: $PUBLIC_KEY"
            break
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    else
        if [[ "$RESPONSE" == *"\"success\":true"* ]]; then
            echo "✅ SUCCESS"
            break
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    fi
done

echo ""
echo "3. Testing Transaction Signing..."
echo "--------------------------------"

SIGN_PAYLOAD=$(cat <<EOF
{
    "user_id": "$USER_ID",
    "message_hash": "$MESSAGE_HASH",
    "transaction_data": "sample_transaction_data"
}
EOF
)

echo "Step 1: Signing initialization"
for port in 8001 8002 8003; do
    echo -n "Node $port: "
    RESPONSE=$(curl -s -X POST "http://localhost:$port/agg-send-step1" \
        -H "Content-Type: application/json" \
        -d "$SIGN_PAYLOAD")
    
    if command -v jq >/dev/null 2>&1; then
        if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
            echo "✅ SUCCESS"
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    else
        if [[ "$RESPONSE" == *"\"success\":true"* ]]; then
            echo "✅ SUCCESS"
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    fi
done

sleep 1

echo ""
echo "Step 2: Signature generation"
for port in 8001 8002 8003; do
    echo -n "Node $port: "
    RESPONSE=$(curl -s -X POST "http://localhost:$port/agg-send-step2" \
        -H "Content-Type: application/json" \
        -d "$SIGN_PAYLOAD")
    
    if command -v jq >/dev/null 2>&1; then
        if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
            SIGNATURE=$(echo "$RESPONSE" | jq -r '.signature')
            echo "✅ SUCCESS - Signature: $SIGNATURE"
            break
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    else
        if [[ "$RESPONSE" == *"\"success\":true"* ]]; then
            echo "✅ SUCCESS"
            break
        else
            echo "❌ FAILED - $RESPONSE"
        fi
    fi
done

echo ""
echo "🎉 MPC Integration Test Complete!"