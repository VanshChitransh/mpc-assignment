#!/bin/bash

echo "🚀 Starting MPC Cluster for Phase 3"
echo "=================================="

# Create data directories
mkdir -p mpc/data/node1
mkdir -p mpc/data/node2  
mkdir -p mpc/data/node3

# Check if MPC binary exists
if [ ! -f "mpc/target/release/mpc" ] && [ ! -f "mpc/target/debug/mpc" ]; then
    echo "Building MPC nodes first..."
    cd mpc
    cargo build --release || cargo build
    cd ..
fi

# Start MPC nodes in background
cd mpc

echo "Starting MPC Node 1..."
NODE_ID=1 BIND_ADDRESS=127.0.0.1:8001 DATA_DIR=./data/node1 \
  PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003 \
  RUST_LOG=mpc=info,frost_ed25519=info \
  cargo run --release 2>/dev/null || cargo run &
NODE1_PID=$!

sleep 2

echo "Starting MPC Node 2..."  
NODE_ID=2 BIND_ADDRESS=127.0.0.1:8002 DATA_DIR=./data/node2 \
  PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003 \
  RUST_LOG=mpc=info,frost_ed25519=info \
  cargo run --release 2>/dev/null || cargo run &
NODE2_PID=$!

sleep 2

echo "Starting MPC Node 3..."
NODE_ID=3 BIND_ADDRESS=127.0.0.1:8003 DATA_DIR=./data/node3 \
  PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003 \
  RUST_LOG=mpc=info,frost_ed25519=info \
  cargo run --release 2>/dev/null || cargo run &
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

# Create a PID file for easy cleanup
echo "$NODE1_PID $NODE2_PID $NODE3_PID" > mpc_cluster.pids

# Wait for Ctrl+C
trap "echo 'Stopping MPC cluster...'; kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null; rm -f mpc_cluster.pids; exit" INT
echo "Press Ctrl+C to stop all nodes"
wait