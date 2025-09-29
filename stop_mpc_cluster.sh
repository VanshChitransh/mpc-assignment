#!/bin/bash

echo "🛑 Stopping MPC Cluster"
echo "======================"

# Try to read PIDs from file first
if [ -f "mpc_cluster.pids" ]; then
    echo "Reading PIDs from mpc_cluster.pids..."
    PIDS=$(cat mpc_cluster.pids)
    echo "Found PIDs: $PIDS"
    
    for PID in $PIDS; do
        if kill -0 $PID 2>/dev/null; then
            echo "Stopping process $PID..."
            kill $PID
        else
            echo "Process $PID already stopped"
        fi
    done
    
    # Clean up PID file
    rm -f mpc_cluster.pids
else
    echo "No PID file found, trying to kill by process name..."
    
    # Kill any running MPC processes
    if pkill -f "target.*mpc"; then
        echo "Stopped MPC processes by name"
    else
        echo "No MPC processes found running"
    fi
fi

# Give processes time to stop gracefully
sleep 2

# Force kill if any are still running
echo "Checking for remaining processes..."
REMAINING=$(pgrep -f "target.*mpc" | wc -l)
if [ $REMAINING -gt 0 ]; then
    echo "Force stopping $REMAINING remaining processes..."
    pkill -9 -f "target.*mpc"
fi

echo ""
echo "✅ MPC Cluster stopped"
echo ""
echo "You can verify no nodes are running with:"
echo "curl -s http://localhost:8001/health 2>/dev/null || echo 'Node 1 stopped'"
echo "curl -s http://localhost:8002/health 2>/dev/null || echo 'Node 2 stopped'"  
echo "curl -s http://localhost:8003/health 2>/dev/null || echo 'Node 3 stopped'"