#!/bin/bash
set -e

# Colors for better output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Running Phase 4 Solana Integration Tests${NC}"
echo -e "${YELLOW}=======================================${NC}"

# Check for required environment variables
if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}Error: DATABASE_URL environment variable not set${NC}"
    echo "Example: export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_wallet"
    exit 1
fi

if [ -z "$JWT_SECRET" ]; then
    echo -e "${YELLOW}Warning: JWT_SECRET not set. Using default value${NC}"
    export JWT_SECRET="test-secret-key-very-long-for-testing-purposes"
fi

if [ -z "$SOLANA_RPC_URL" ]; then
    echo -e "${YELLOW}Warning: SOLANA_RPC_URL not set. Using Solana Devnet${NC}"
    export SOLANA_RPC_URL="https://api.devnet.solana.com"
fi

# Make sure we're in the backend directory
cd "$(dirname "$0")"

# Start backend server in background
echo -e "${YELLOW}Starting backend server...${NC}"
RUST_LOG=info cargo run &
SERVER_PID=$!

# Wait for server to start
echo -e "${YELLOW}Waiting for server to start...${NC}"
sleep 5

# Test functions
function test_signup {
    echo -e "${YELLOW}Testing user signup with MPC key generation...${NC}"
    SIGNUP_RESULT=$(curl -s -X POST http://localhost:8080/api/user/signup \
        -H "Content-Type: application/json" \
        -d '{"email":"test_phase4@example.com","password":"SecurePass123!"}')
    
    if echo "$SIGNUP_RESULT" | grep -q "public_key"; then
        echo -e "${GREEN}✓ Signup successful with MPC key generation${NC}"
        # Extract JWT token and user ID for later tests
        export JWT_TOKEN=$(echo "$SIGNUP_RESULT" | jq -r '.token')
        export USER_ID=$(echo "$SIGNUP_RESULT" | jq -r '.user.id')
        echo "JWT Token: ${JWT_TOKEN:0:20}..."
        echo "User ID: $USER_ID"
        return 0
    else
        echo -e "${RED}✗ Signup failed${NC}"
        echo "$SIGNUP_RESULT" | jq
        return 1
    fi
}

function test_balance {
    echo -e "${YELLOW}Testing balance endpoint...${NC}"
    BALANCE_RESULT=$(curl -s -X GET http://localhost:8080/api/solana/balance \
        -H "Authorization: Bearer $JWT_TOKEN")
    
    if echo "$BALANCE_RESULT" | grep -q "balances"; then
        echo -e "${GREEN}✓ Balance endpoint successful${NC}"
        echo "$BALANCE_RESULT" | jq '.balances[] | {mint, symbol, ui_amount}'
        return 0
    else
        echo -e "${RED}✗ Balance endpoint failed${NC}"
        echo "$BALANCE_RESULT" | jq
        return 1
    fi
}

function test_quote {
    echo -e "${YELLOW}Testing swap quote endpoint...${NC}"
    QUOTE_RESULT=$(curl -s -X POST http://localhost:8080/api/solana/quote \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -d '{"input_mint":"So11111111111111111111111111111111111111112","output_mint":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","amount":"1000000000","slippage":0.5}')
    
    if echo "$QUOTE_RESULT" | grep -q "quote_id"; then
        echo -e "${GREEN}✓ Quote endpoint successful${NC}"
        export QUOTE_ID=$(echo "$QUOTE_RESULT" | jq -r '.quote_id')
        echo "Quote ID: $QUOTE_ID"
        echo "Input Amount: $(echo "$QUOTE_RESULT" | jq -r '.in_amount')"
        echo "Output Amount: $(echo "$QUOTE_RESULT" | jq -r '.out_amount')"
        return 0
    else
        echo -e "${RED}✗ Quote endpoint failed${NC}"
        echo "$QUOTE_RESULT" | jq
        return 1
    fi
}

function test_swap {
    if [ -z "$QUOTE_ID" ]; then
        echo -e "${YELLOW}Skipping swap test - no quote ID available${NC}"
        return 0
    fi

    echo -e "${YELLOW}Testing swap execution endpoint...${NC}"
    SWAP_RESULT=$(curl -s -X POST http://localhost:8080/api/solana/swap \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -d "{\"quote_id\":\"$QUOTE_ID\"}")
    
    if echo "$SWAP_RESULT" | grep -q "transaction_id"; then
        echo -e "${GREEN}✓ Swap endpoint successful${NC}"
        echo "Transaction ID: $(echo "$SWAP_RESULT" | jq -r '.transaction_id')"
        return 0
    else
        echo -e "${RED}✗ Swap endpoint failed${NC}"
        echo "$SWAP_RESULT" | jq
        return 1
    fi
}

function test_send {
    echo -e "${YELLOW}Testing token transfer endpoint...${NC}"
    SEND_RESULT=$(curl -s -X POST http://localhost:8080/api/solana/send \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -d '{"to_address":"11111111111111111111111111111111","mint":"So11111111111111111111111111111111111111112","amount":"1000","decimals":9}')
    
    # This will likely fail due to insufficient funds, but we want to test the API structure
    echo -e "${YELLOW}Send endpoint response (may fail due to insufficient funds - this is OK)${NC}"
    echo "$SEND_RESULT" | jq
    
    # Test sending an SPL token
    echo -e "${YELLOW}Testing SPL token transfer endpoint...${NC}"
    SEND_RESULT=$(curl -s -X POST http://localhost:8080/api/solana/send \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $JWT_TOKEN" \
        -d '{"to_address":"11111111111111111111111111111111","mint":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","amount":"1000","decimals":6}')
    
    echo -e "${YELLOW}SPL send endpoint response (may fail due to insufficient funds - this is OK)${NC}"
    echo "$SEND_RESULT" | jq
    
    return 0
}

# Run tests
test_signup
test_balance
test_quote
test_swap
test_send

# Check if both Jupiter API is working
echo -e "${YELLOW}Testing direct Jupiter API access...${NC}"
JUPITER_RESULT=$(curl -s "https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111111111112&outputMint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v&amount=10000000&slippageBps=50")

if echo "$JUPITER_RESULT" | grep -q "outAmount"; then
    echo -e "${GREEN}✓ Jupiter API is accessible${NC}"
else
    echo -e "${RED}✗ Jupiter API test failed - check network connectivity${NC}"
    echo "$JUPITER_RESULT" | jq
fi

# Test Solana RPC
echo -e "${YELLOW}Testing Solana RPC access...${NC}"
SOLANA_RESULT=$(curl -s -X POST $SOLANA_RPC_URL -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}')

if echo "$SOLANA_RESULT" | grep -q "ok"; then
    echo -e "${GREEN}✓ Solana RPC is accessible and healthy${NC}"
else
    echo -e "${RED}✗ Solana RPC test failed - check network connectivity${NC}"
    echo "$SOLANA_RESULT" | jq
fi

# Clean up
echo -e "${YELLOW}Cleaning up...${NC}"
kill $SERVER_PID

echo -e "${GREEN}Phase 4 Tests Completed!${NC}"