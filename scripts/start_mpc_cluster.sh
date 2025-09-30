#!/bin/bash

# Kill any existing MPC nodes
pkill -f "target/debug/mpc" || true
sleep 2

# Create data directories
mkdir -p data/node1 data/node2 data/node3

# Start MPC Node 1
NODE_ID=1 \
DATA_DIR=./data/node1 \
BIND_ADDRESS=127.0.0.1:8001 \
PEER_NODES=http://127.0.0.1:8002,http://127.0.0.1:8003 \
cargo run --manifest-path mpc/Cargo.toml > logs/mpc_node1.log 2>&1 &

echo "Started MPC Node 1 (PID: $!)"

# Start MPC Node 2
NODE_ID=2 \
DATA_DIR=./data/node2 \
BIND_ADDRESS=127.0.0.1:8002 \
PEER_NODES=http://127.0.0.1:8001,http://127.0.0.1:8003 \
cargo run --manifest-path mpc/Cargo.toml > logs/mpc_node2.log 2>&1 &

echo "Started MPC Node 2 (PID: $!)"

# Start MPC Node 3
NODE_ID=3 \
DATA_DIR=./data/node3 \
BIND_ADDRESS=127.0.0.1:8003 \
PEER_NODES=http://127.0.0.1:8001,http://127.0.0.1:8002 \
cargo run --manifest-path mpc/Cargo.toml > logs/mpc_node3.log 2>&1 &

echo "Started MPC Node 3 (PID: $!)"

echo ""
echo "Waiting for nodes to start..."
sleep 5

# Check health
echo ""
echo "Checking node health:"
curl -s http://127.0.0.1:8001/health | jq '.' || echo "Node 1 health check failed"
curl -s http://127.0.0.1:8002/health | jq '.' || echo "Node 2 health check failed"
curl -s http://127.0.0.1:8003/health | jq '.' || echo "Node 3 health check failed"

echo ""
echo "MPC Cluster is running!"
echo "Logs: logs/mpc_node*.log"
