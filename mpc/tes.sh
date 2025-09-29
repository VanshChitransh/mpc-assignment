# mpc/.env.node1
NODE_ID=1
BIND_ADDRESS=127.0.0.1:8001
DATA_DIR=./data/node1
PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
RUST_LOG=mpc=info,frost_ed25519=info

# mpc/.env.node2  
NODE_ID=2
BIND_ADDRESS=127.0.0.1:8002
DATA_DIR=./data/node2
PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
RUST_LOG=mpc=info,frost_ed25519=info

# mpc/.env.node3
NODE_ID=3
BIND_ADDRESS=127.0.0.1:8003
DATA_DIR=./data/node3
PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
RUST_LOG=mpc=info,frost_ed25519=info

# Backend environment update (add to backend/.env)
MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
MPC_THRESHOLD=2

# Root project startup script: start_mpc_cluster.sh
#!/bin/bash

echo "🚀 Starting MPC Cluster for Phase 3"
echo "=================================="

# Create data directories
mkdir -p mpc/data/node1
mkdir -p mpc/data/node2  
mkdir -p mpc/data/node3

# Start MPC nodes in background
cd mpc

echo "Starting MPC Node 1..."
NODE_ID=1 BIND_ADDRESS=127.0.0.1:8001 DATA_DIR=./data/node1 \
  PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003 \
  cargo run &
NODE1_PID=$!

sleep 2

echo "Starting MPC Node 2..."  
NODE_ID=2 BIND_ADDRESS=127.0.0.1:8002 DATA_DIR=./data/node2 \
  PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003 \
  cargo run &
NODE2_PID=$!

sleep 2

echo "Starting MPC Node 3..."
NODE_ID=3 BIND_ADDRESS=127.0.0.1:8003 DATA_DIR=./data/node3 \
  PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003 \
  cargo run &
NODE3_PID=$!

echo ""
echo "✅ MPC Cluster started successfully!"
echo "Node 1: http://localhost:8001/health"
echo "Node 2: http://localhost:8002/health"  
echo "Node 3: http://localhost:8003/health"
echo ""
echo "PIDs: Node1=$NODE1_PID Node2=$NODE2_PID Node3=$NODE3_PID"
echo ""
echo "To stop the cluster, run: kill $NODE1_PID $NODE2_PID $NODE3_PID"
echo "Or use: pkill -f 'target.*mpc'"

cd ..

echo ""
echo "Now you can start the backend server:"
echo "cd backend && cargo run"

# Wait for Ctrl+C
trap "echo 'Stopping MPC cluster...'; kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null; exit" INT
echo "Press Ctrl+C to stop all nodes"
wait

# Health check script: check_mpc_health.sh
#!/bin/bash

echo "🔍 Checking MPC Cluster Health"
echo "=============================="

check_node() {
    local node_url=$1
    local node_name=$2
    
    if curl -s --connect-timeout 3 "$node_url/health" > /dev/null; then
        echo "✅ $node_name: HEALTHY ($node_url)"
        curl -s "$node_url/health" | jq .
    else
        echo "❌ $node_name: UNREACHABLE ($node_url)"
    fi
    echo ""
}

check_node "http://localhost:8001" "Node 1"
check_node "http://localhost:8002" "Node 2" 
check_node "http://localhost:8003" "Node 3"

echo "Cluster Status Summary:"
HEALTHY_COUNT=0
for port in 8001 8002 8003; do
    if curl -s --connect-timeout 1 "http://localhost:$port/health" > /dev/null; then
        ((HEALTHY_COUNT++))
    fi
done

echo "Healthy nodes: $HEALTHY_COUNT/3"
if [ $HEALTHY_COUNT -ge 2 ]; then
    echo "✅ Cluster is OPERATIONAL (threshold: 2)"
else
    echo "❌ Cluster is NOT OPERATIONAL (need at least 2 nodes)"
fi

# Test script: test_mpc_integration.sh  
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
    
    if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
        echo "✅ SUCCESS"
    else
        echo "❌ FAILED - $RESPONSE"
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
    
    if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
        PUBLIC_KEY=$(echo "$RESPONSE" | jq -r '.public_key')
        echo "✅ SUCCESS - Public Key: $PUBLIC_KEY"
        break
    else
        echo "❌ FAILED - $RESPONSE"
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
    
    if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
        echo "✅ SUCCESS"
    else
        echo "❌ FAILED - $RESPONSE"
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
    
    if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
        SIGNATURE=$(echo "$RESPONSE" | jq -r '.signature')
        echo "✅ SUCCESS - Signature: $SIGNATURE"
        break
    else
        echo "❌ FAILED - $RESPONSE"
    fi
done

echo ""
echo "🎉 MPC Integration Test Complete!"