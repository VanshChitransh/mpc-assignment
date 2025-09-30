#!/bin/bash

# Test runner for Step 3.1 - MPC Client Service
# This script runs comprehensive tests to validate the MPC client implementation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║        Step 3.1 - MPC Client Service Test Suite          ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check if MPC cluster is running
check_mpc_cluster() {
    echo -e "${BLUE}[1/5] Checking MPC Cluster Status...${NC}"
    
    local all_healthy=true
    
    for port in 8001 8002 8003; do
        if curl -s "http://localhost:$port/health" > /dev/null 2>&1; then
            echo -e "  ${GREEN}✓${NC} Node on port $port is healthy"
        else
            echo -e "  ${RED}✗${NC} Node on port $port is not responding"
            all_healthy=false
        fi
    done
    
    if [ "$all_healthy" = false ]; then
        echo -e "${YELLOW}"
        echo "⚠️  Warning: Not all MPC nodes are running!"
        echo "Some tests may fail. To start the cluster, run:"
        echo "  ./start_mpc_cluster.sh"
        echo -e "${NC}"
        read -p "Continue anyway? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        echo -e "${GREEN}✓ All MPC nodes are healthy${NC}\n"
    fi
}

# Run cargo tests
run_tests() {
    echo -e "${BLUE}[2/5] Compiling Backend with MPC Client...${NC}"
    
    cd backend
    
    if ! cargo build --tests 2>&1 | tail -20; then
        echo -e "${RED}✗ Compilation failed${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Compilation successful${NC}\n"
    
    echo -e "${BLUE}[3/5] Running Test Suite...${NC}"
    echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}\n"
    
    # Run individual tests with nice output
    local tests=(
        "test_01_generate_key_success"
        "test_02_sign_message_success"
        "test_03_sign_transaction_success"
        "test_04_threshold_availability_check"
        "test_05_health_check_api"
        "test_06_get_cluster_status"
        "test_07_round_robin_load_balancing"
        "test_08_health_based_load_balancing"
        "test_09_random_load_balancing"
        "test_10_retry_on_transient_failure"
        "test_11_custom_retry_config"
        "test_12_insufficient_nodes_error"
        "test_13_network_timeout_handling"
        "test_14_concurrent_operations"
        "test_15_sequential_operations_performance"
    )
    
    local passed=0
    local failed=0
    
    for test in "${tests[@]}"; do
        echo -e "${MAGENTA}Running: $test${NC}"
        if cargo test --test test_step_3_1_complete "$test" -- --nocapture 2>&1; then
            ((passed++))
        else
            ((failed++))
        fi
        echo
    done
    
    echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}[4/5] Individual Test Results:${NC}"
    echo "  Passed: $passed/${#tests[@]}"
    echo "  Failed: $failed/${#tests[@]}"
    echo
}

# Run final validation
run_final_validation() {
    echo -e "${BLUE}[5/5] Running Final Validation...${NC}"
    echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}\n"
    
    if cargo test --test test_step_3_1_complete test_99_complete_step_3_1_validation -- --nocapture; then
        echo -e "\n${GREEN}════════════════════════════════════════════════════════════${NC}"
        echo -e "${GREEN}✓ STEP 3.1 VALIDATION PASSED!${NC}"
        echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}\n"
        
        echo -e "${CYAN}Summary of Implemented Features:${NC}"
        echo
        echo -e "${GREEN}✓${NC} Core MPC Operations"
        echo "  • generate_key(user_id) -> public_key"
        echo "  • sign_message(user_id, message_hex) -> signature"
        echo "  • sign_transaction(user_id, tx_hash, tx_data) -> signature"
        echo
        echo -e "${GREEN}✓${NC} Health Monitoring"
        echo "  • health_check() -> node_status (PUBLIC API)"
        echo "  • check_threshold_availability() -> bool"
        echo "  • get_cluster_status() -> ClusterStatus"
        echo
        echo -e "${GREEN}✓${NC} Load Balancing"
        echo "  • Round-robin node selection"
        echo "  • Health-based routing"
        echo "  • Random distribution"
        echo
        echo -e "${GREEN}✓${NC} Retry Logic"
        echo "  • Exponential backoff"
        echo "  • Configurable retry attempts"
        echo "  • Node fallback mechanisms"
        echo
        echo -e "${GREEN}✓${NC} Circuit Breaker"
        echo "  • Failure counting per node"
        echo "  • Automatic circuit opening/closing"
        echo "  • Timeout-based recovery"
        echo
        echo -e "${GREEN}✓${NC} Error Handling"
        echo "  • Network timeouts"
        echo "  • Node unavailability"
        echo "  • Insufficient nodes scenarios"
        echo "  • Partial responses"
        echo
        echo -e "${CYAN}Next Steps:${NC}"
        echo "  → Proceed to Step 3.2: Complete User Routes with MPC"
        echo "  → Implement signup workflow with MPC key generation"
        echo "  → Integrate MPC signing into user operations"
        echo
        return 0
    else
        echo -e "\n${RED}════════════════════════════════════════════════════════════${NC}"
        echo -e "${RED}✗ STEP 3.1 VALIDATION FAILED${NC}"
        echo -e "${RED}════════════════════════════════════════════════════════════${NC}\n"
        
        echo -e "${YELLOW}Troubleshooting:${NC}"
        echo "  1. Ensure all 3 MPC nodes are running (./start_mpc_cluster.sh)"
        echo "  2. Check MPC node logs for errors"
        echo "  3. Verify network connectivity to localhost:8001-8003"
        echo "  4. Review test output above for specific failures"
        echo
        return 1
    fi
}

# Generate test report
generate_report() {
    echo -e "${BLUE}Generating Test Report...${NC}"
    
    local report_file="test_results_step_3_1.txt"
    
    {
        echo "Step 3.1 Test Report"
        echo "===================="
        echo "Date: $(date)"
        echo "User: $(whoami)"
        echo ""
        echo "MPC Cluster Status:"
        for port in 8001 8002 8003; do
            if curl -s "http://localhost:$port/health" > /dev/null 2>&1; then
                echo "  Node $port: HEALTHY"
            else
                echo "  Node $port: UNHEALTHY"
            fi
        done
        echo ""
        echo "For detailed test output, see above."
    } > "$report_file"
    
    echo -e "${GREEN}✓ Report saved to: $report_file${NC}\n"
}

# Main execution
main() {
    # Change to project root
    cd "$(dirname "$0")"
    
    # Check MPC cluster
    check_mpc_cluster
    
    # Run tests
    run_tests
    
    # Run final validation
    if run_final_validation; then
        generate_report
        exit 0
    else
        generate_report
        exit 1
    fi
}

# Run main function
main "$@"