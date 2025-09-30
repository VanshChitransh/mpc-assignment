#!/bin/bash

echo "Stopping Phase 3 Test Environment..."

if [ -f backend.pid ]; then
    BACKEND_PID=$(cat backend.pid)
    if ps -p $BACKEND_PID > /dev/null 2>&1; then
        kill $BACKEND_PID
        echo "✓ Backend server stopped (PID: $BACKEND_PID)"
    fi
    rm backend.pid
else
    # Fallback: kill by port
    BACKEND_PID=$(lsof -ti:8080)
    if [ ! -z "$BACKEND_PID" ]; then
        kill $BACKEND_PID
        echo "✓ Backend server stopped"
    fi
fi

echo "Test environment stopped"