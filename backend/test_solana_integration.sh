#!/bin/bash

# Set up environment for testing
export RUST_LOG=info
export SOLANA_RPC_URL=https://api.devnet.solana.com
export SOLANA_COMMITMENT=confirmed
export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_wallet
export JWT_SECRET=test-secret-for-integration-tests-only

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Starting Solana Integration Tests...${NC}"

# Check if the blockchain module compiles
echo -e "\n${YELLOW}Checking if the blockchain module compiles...${NC}"
cargo check --lib
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Blockchain module compiles successfully!${NC}"
else
    echo -e "${RED}❌ Blockchain module has compilation errors!${NC}"
    exit 1
fi

# Build the binary
echo -e "\n${YELLOW}Building the backend binary...${NC}"
cargo build
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Backend binary built successfully!${NC}"
else
    echo -e "${RED}❌ Backend binary has compilation errors!${NC}"
    exit 1
fi

# Run unit tests for the Solana blockchain module
echo -e "\n${YELLOW}Running Solana blockchain unit tests...${NC}"
cargo test blockchain::solana::tests

# Check the exit status
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Solana blockchain unit tests passed!${NC}"
else
    echo -e "${RED}✗ Solana blockchain unit tests failed!${NC}"
    exit 1
fi

# Test the API endpoints
echo -e "\n${YELLOW}Testing API endpoints (requires running server)...${NC}"
echo -e "${YELLOW}Make sure the backend server is running before continuing${NC}"
read -p "Press enter to continue or Ctrl+C to cancel..."

# Test address derivation
echo -e "\n${YELLOW}Testing /api/v1/solana/address endpoint...${NC}"
curl -X POST http://localhost:8080/api/v1/solana/address \
  -H "Content-Type: application/json" \
  -d '{
    "public_key": "0000000000000000000000000000000000000000000000000000000000000000"
  }'

# Print success message
echo -e "\n\n${GREEN}All Solana integration tests completed!${NC}"
echo -e "${YELLOW}Note: Some tests may require proper authentication.${NC}"
echo -e "${YELLOW}To fully test the API endpoints, use a valid JWT token in your requests.${NC}"

exit 0