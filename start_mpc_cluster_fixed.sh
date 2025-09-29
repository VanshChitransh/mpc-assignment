#!/bin/bash

echo "🚀 Starting MPC Cluster (Fixed Version)"
echo "======================================="

# Kill any existing MPC processes
pkill -f 'target.*mpc' 2>/dev/null || true
sleep 1

# Create data directories
mkdir -p mpc/data/node1
mkdir -p mpc/data/node2  
mkdir -p mpc/data/node3

# Build MPC binary once
echo "Building MPC binary..."
cd mpc
cargo build --release 2>/dev/null || cargo build
cd ..

# Function to start a node
start_node() {
    local node_id=$1
    local port=$2
    local data_dir=$3
    
    echo "Starting MPC Node $node_id on port $port..."
    
    # Start in background with proper environment
    NODE_ID=$node_id \
    BIND_ADDRESS=127.0.0.1:$port \
    DATA_DIR=$data_dir \
    PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003 \
    RUST_LOG=mpc=info,frost_ed25519=info \
    cargo run --manifest-path mpc/Cargo.toml --release 2>/dev/null || \
    cargo run --manifest-path mpc/Cargo.toml &
    
    local pid=$!
    echo "Node $node_id started with PID: $pid"
    return $pid
}

# Start all three nodes
start_node 1 8001 mpc/data/node1
NODE1_PID=$!

sleep 3

start_node 2 8002 mpc/data/node2  
NODE2_PID=$!

sleep 3

start_node 3 8003 mpc/data/node3
NODE3_PID=$!

sleep 2

echo ""
echo "✅ MPC Cluster started successfully!"
echo "Node 1: http://localhost:8001/health (PID: $NODE1_PID)"
echo "Node 2: http://localhost:8002/health (PID: $NODE2_PID)"  
echo "Node 3: http://localhost:8003/health (PID: $NODE3_PID)"
echo ""

# Test health endpoints
echo "Testing cluster health..."
for port in 8001 8002 8003; do
    echo -n "Node $port: "
    if curl -s http://localhost:$port/health > /dev/null 2>&1; then
        echo "✅ HEALTHY"
    else
        echo "❌ UNHEALTHY"
    fi
done

echo ""
echo "To stop the cluster, run: kill $NODE1_PID $NODE2_PID $NODE3_PID"
echo "Or use: pkill -f 'target.*mpc'"

# Create a PID file for easy cleanup
echo "$NODE1_PID $NODE2_PID $NODE3_PID" > mpc_cluster.pids

# Wait for Ctrl+C
trap "echo 'Stopping MPC cluster...'; kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null; rm -f mpc_cluster.pids; exit" INT
echo "Press Ctrl+C to stop all nodes"
wait
