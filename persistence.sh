# Test persistence
echo "🔄 Testing Key Persistence After Restart"
echo "========================================"

# Store the user ID and public key from the previous test
USER_ID="550e8400-e29b-41d4-a716-446655440000"
ORIGINAL_PUBLIC_KEY="65604950760af1cb983be889e21a673998f3fd774b443d39d3ba7289083a62da"

# Kill any running MPC nodes
pkill -f "target.*mpc" 2>/dev/null
sleep 2

echo "Starting fresh MPC nodes (data should persist)..."
cd mpc

# Start node 1
NODE_ID=1 BIND_ADDRESS=127.0.0.1:8001 DATA_DIR=./data/node1 cargo run > node1.log 2>&1 &
NODE1_PID=$!
sleep 3

cd ..

echo "Testing if keys persisted..."
RESPONSE=$(curl -s -X POST "http://localhost:8001/aggregate-keys" \
    -H "Content-Type: application/json" \
    -d "{\"user_id\":\"$USER_ID\"}")

if echo "$RESPONSE" | grep -q "$ORIGINAL_PUBLIC_KEY"; then
    echo "✅ Keys persisted! Same public key retrieved: $ORIGINAL_PUBLIC_KEY"
else
    echo "❌ Key persistence failed or public key changed"
    echo "Response: $RESPONSE"
fi

# Test signing with persisted key
echo ""
echo "Testing signing with persisted key..."
MESSAGE_HASH=$(echo -n "persistence test" | sha256sum | cut -d' ' -f1)

RESPONSE=$(curl -s -X POST "http://localhost:8001/agg-send-step1" \
    -H "Content-Type: application/json" \
    -d "{\"user_id\":\"$USER_ID\",\"message_hash\":\"$MESSAGE_HASH\",\"transaction_data\":\"test\"}")

if echo "$RESPONSE" | grep -q '"success":true'; then
    echo "✅ Signing step 1 with persisted key successful"
    
    RESPONSE=$(curl -s -X POST "http://localhost:8001/agg-send-step2" \
        -H "Content-Type: application/json" \
        -d "{\"user_id\":\"$USER_ID\",\"message_hash\":\"$MESSAGE_HASH\",\"transaction_data\":\"test\"}")
    
    if echo "$RESPONSE" | grep -q '"success":true'; then
        echo "✅ Signing step 2 with persisted key successful"
    fi
fi

# Cleanup
kill $NODE1_PID 2>/dev/null