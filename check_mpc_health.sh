#!/bin/bash

echo "🔍 Checking MPC Cluster Health"
echo "=============================="

check_node() {
    local node_url=$1
    local node_name=$2
    
    if curl -s --connect-timeout 3 "$node_url/health" > /dev/null; then
        echo "✅ $node_name: HEALTHY ($node_url)"
        if command -v jq >/dev/null 2>&1; then
            curl -s "$node_url/health" | jq .
        else
            curl -s "$node_url/health"
        fi
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