#!/bin/bash

echo "🧪 Testing MPC Step 2.1 Implementation"
echo "======================================"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Start MPC nodes
echo "Starting MPC nodes..."
cd mpc

# Kill any existing MPC processes
pkill -f "target.*mpc" 2>/dev/null
sleep 1

# Start node 1
NODE_ID=1 BIND_ADDRESS=127.0.0.1:8001 DATA_DIR=./data/node1 cargo run > node1.log 2>&1 &
NODE1_PID=$!
sleep 2

# Start node 2
NODE_ID=2 BIND_ADDRESS=127.0.0.1:8002 DATA_DIR=./data/node2 cargo run > node2.log 2>&1 &
NODE2_PID=$!
sleep 2

# Start node 3
NODE_ID=3 BIND_ADDRESS=127.0.0.1:8003 DATA_DIR=./data/node3 cargo run > node3.log 2>&1 &
NODE3_PID=$!
sleep 3

cd ..

echo ""
echo "Testing health endpoints..."
for port in 8001 8002 8003; do
    if curl -s "http://localhost:$port/health" > /dev/null; then
        echo -e "${GREEN}✅ Node on port $port is healthy${NC}"
    else
        echo -e "${RED}❌ Node on port $port is not responding${NC}"
    fi
done

echo ""
echo "Testing key generation..."
USER_ID="550e8400-e29b-41d4-a716-446655440000"

RESPONSE=$(curl -s -X POST "http://localhost:8001/generate" \
    -H "Content-Type: application/json" \
    -d "{\"user_id\":\"$USER_ID\",\"threshold\":2,\"total_parties\":3}")

if echo "$RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}✅ Key generation successful${NC}"
    PUBLIC_KEY=$(echo "$RESPONSE" | grep -o '"public_key":"[^"]*' | cut -d'"' -f4)
    echo "Public key: $PUBLIC_KEY"
else
    echo -e "${RED}❌ Key generation failed${NC}"
    echo "$RESPONSE"
fi

echo ""
echo "Testing key aggregation..."
RESPONSE=$(curl -s -X POST "http://localhost:8001/aggregate-keys" \
    -H "Content-Type: application/json" \
    -d "{\"user_id\":\"$USER_ID\"}")

if echo "$RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}✅ Key aggregation successful${NC}"
else
    echo -e "${RED}❌ Key aggregation failed${NC}"
    echo "$RESPONSE"
fi

echo ""
echo "Testing signing step 1..."
MESSAGE_HASH=$(echo -n "test message" | sha256sum | cut -d' ' -f1)
RESPONSE=$(curl -s -X POST "http://localhost:8001/agg-send-step1" \
    -H "Content-Type: application/json" \
    -d "{\"user_id\":\"$USER_ID\",\"message_hash\":\"$MESSAGE_HASH\",\"transaction_data\":\"test\"}")

if echo "$RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}✅ Signing step 1 successful${NC}"
else
    echo -e "${RED}❌ Signing step 1 failed${NC}"
    echo "$RESPONSE"
fi

echo ""
echo "Testing signing step 2..."
RESPONSE=$(curl -s -X POST "http://localhost:8001/agg-send-step2" \
    -H "Content-Type: application/json" \
    -d "{\"user_id\":\"$USER_ID\",\"message_hash\":\"$MESSAGE_HASH\",\"transaction_data\":\"test\"}")

if echo "$RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}✅ Signing step 2 successful${NC}"
    SIGNATURE=$(echo "$RESPONSE" | grep -o '"signature":"[^"]*' | cut -d'"' -f4)
    echo "Signature: $SIGNATURE"
else
    echo -e "${RED}❌ Signing step 2 failed${NC}"
    echo "$RESPONSE"
fi

echo ""
echo "Cleaning up..."
kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null

echo ""
echo -e "${GREEN}🎉 Test complete!${NC}"
