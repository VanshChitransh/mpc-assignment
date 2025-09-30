#!/bin/bash

set -e

echo "Testing MPC Cluster"
echo "==================="
echo ""

USER_ID="550e8400-e29b-41d4-a716-446655440000"

# Test keygen on all 3 nodes
echo "Test 1: Key Generation"
echo "----------------------"

for node in 1 2 3; do
    echo "Testing node $node..."
    RESPONSE=$(curl -s -X POST http://127.0.0.1:800${node}/keygen \
        -H "Content-Type: application/json" \
        -d "{\"user_id\":\"$USER_ID\",\"threshold\":2,\"total_parties\":3}")
    
    echo "Node $node response: $RESPONSE"
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
    if [ "$SUCCESS" = "true" ]; then
        echo "Node $node: PASS"
    else
        echo "Node $node: FAIL"
        exit 1
    fi
    echo ""
done

echo "All tests PASSED!"
