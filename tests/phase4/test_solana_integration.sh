#!/bin/bash

# Test Solana Integration on Devnet
# This script tests the Phase 4, Step 4.1 Solana blockchain integration

set -e

echo "================================"
echo "Phase 4.1: Solana Integration Tests"
echo "================================"
echo ""

# Set environment to Devnet
export SOLANA_RPC_URL="https://api.devnet.solana.com"
export SOLANA_COMMITMENT="confirmed"
export TEST_DATABASE_URL="postgresql://newuser:new_secure_password@localhost:5432/newdb_test"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Test Environment Configuration:"
echo "- RPC URL: $SOLANA_RPC_URL"
echo "- Commitment: $SOLANA_COMMITMENT"
echo "- Database: $TEST_DATABASE_URL"
echo ""

# Function to print test results
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ $2${NC}"
    else
        echo -e "${RED}✗ $2${NC}"
        exit 1
    fi
}

# Test 1: Unit tests for blockchain module
echo -e "${YELLOW}Test 1: Running blockchain module unit tests...${NC}"
cd backend
cargo test --lib blockchain::solana --no-fail-fast -- --nocapture
print_result $? "Blockchain module unit tests"
echo ""

# Test 2: Integration tests
echo -e "${YELLOW}Test 2: Running Solana integration tests...${NC}"
cargo test solana_integration --no-fail-fast -- --nocapture
print_result $? "Solana integration tests"
echo ""

# Test 3: Address derivation
echo -e "${YELLOW}Test 3: Testing address derivation...${NC}"
cargo test test_derive_solana_address --no-fail-fast -- --nocapture
print_result $? "Address derivation"
echo ""

# Test 4: Address validation
echo -e "${YELLOW}Test 4: Testing address validation...${NC}"
cargo test test_validate_address --no-fail-fast -- --nocapture
print_result $? "Address validation"
echo ""

# Test 5: Transaction building
echo -e "${YELLOW}Test 5: Testing transaction building...${NC}"
cargo test test_build_transaction --no-fail-fast -- --nocapture
print_result $? "Transaction building"
echo ""

# Test 6: Transaction signing
echo -e "${YELLOW}Test 6: Testing transaction signing...${NC}"
cargo test test_sign_transaction --no-fail-fast -- --nocapture
print_result $? "Transaction signing"
echo ""

# Test 7: RPC connectivity (Devnet)
echo -e "${YELLOW}Test 7: Testing Devnet RPC connectivity...${NC}"
cargo test test_get_recent_blockhash_devnet --no-fail-fast -- --nocapture
print_result $? "Devnet RPC connectivity"
echo ""

# Test 8: API endpoints
echo -e "${YELLOW}Test 8: Testing API endpoints...${NC}"
cargo test test_derive_address_success --no-fail-fast -- --nocapture
print_result $? "API endpoint /api/v1/solana/address"
echo ""

# Test 9: Security validation
echo -e "${YELLOW}Test 9: Testing security validation...${NC}"
cargo test test_transfer_endpoint_without_auth --no-fail-fast -- --nocapture
print_result $? "Authentication required for transfers"
echo ""

# Test 10: Invalid input handling
echo -e "${YELLOW}Test 10: Testing invalid input handling...${NC}"
cargo test test_derive_address_invalid_public_key --no-fail-fast -- --nocapture
print_result $? "Invalid public key rejection"
echo ""

# Test 11: Edge cases
echo -e "${YELLOW}Test 11: Testing edge cases...${NC}"
cargo test test_address_validation_edge_cases --no-fail-fast -- --nocapture
print_result $? "Address validation edge cases"
echo ""

# Test 12: Metrics
echo -e "${YELLOW}Test 12: Testing Prometheus metrics...${NC}"
cargo test test_solana_metrics_initialization --no-fail-fast -- --nocapture
print_result $? "Prometheus metrics initialization"
echo ""

# Test 13: Build and compile check
echo -e "${YELLOW}Test 13: Testing build compilation...${NC}"
cd ..
cargo build --manifest-path backend/Cargo.toml --release 2>&1 | grep -i "error" && exit 1 || true
print_result 0 "Backend compilation"
echo ""

echo ""
echo "================================"
echo -e "${GREEN}All Solana Integration Tests Passed!${NC}"
echo "================================"
echo ""
echo "Summary:"
echo "- Address derivation: ✓"
echo "- Address validation: ✓"
echo "- Transaction building: ✓"
echo "- Transaction signing: ✓"
echo "- RPC connectivity: ✓"
echo "- API endpoints: ✓"
echo "- Security validation: ✓"
echo "- Error handling: ✓"
echo "- Observability: ✓"
echo ""
echo "Phase 4, Step 4.1 - Solana Integration: COMPLETE"
