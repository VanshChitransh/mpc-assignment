#!/bin/bash

echo "🚀 Phase 3: MPC Server Implementation Setup"
echo "==========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✅ $2${NC}"
    else
        echo -e "${RED}❌ $2${NC}"
        exit 1
    fi
}

# Check prerequisites
echo "Step 1: Checking Prerequisites"
echo "------------------------------"

# Check if Rust is installed
if command -v cargo >/dev/null 2>&1; then
    print_status 0 "Rust/Cargo is installed"
else
    echo -e "${RED}❌ Rust is not installed. Please install from https://rustup.rs/${NC}"
    exit 1
fi

# Check if we're in the right directory structure
if [ ! -d "backend" ] || [ ! -d "store" ]; then
    echo -e "${RED}❌ Please run this script from the project root directory${NC}"
    echo "Expected structure: ./backend/, ./store/, etc."
    exit 1
fi

print_status 0 "Project structure verified"

echo ""
echo "Step 2: Creating MPC Module Structure"
echo "------------------------------------"

# Create MPC directory structure
mkdir -p mpc/src
mkdir -p mpc/data/node1
mkdir -p mpc/data/node2
mkdir -p mpc/data/node3

print_status 0 "MPC directories created"

# Create Cargo.toml for MPC module
cat > mpc/Cargo.toml << 'EOF'
[package]
name = "mpc"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4.4"
actix-cors = "0.7"
dotenv = "0.15"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "1.0"
anyhow = "1.0"
chrono = { version = "0.4", features = ["serde"] }

# Database
sled = "0.34"

# Serialization  
rmp-serde = "1.1"
hex = "0.4"

# Cryptography - FROST implementation
frost-ed25519 = "2.0"
rand = "0.8"

# HTTP client for node communication
reqwest = { version = "0.11", features = ["json"] }

# Async utilities
futures-util = "0.3"
EOF

print_status 0 "MPC Cargo.toml created"

# Create basic error.rs
cat > mpc/src/error.rs << 'EOF'
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MpcError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Cryptographic error: {0}")]
    CryptographicError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Invalid participant ID: {0}")]
    InvalidParticipantId(String),
    #[error("Insufficient participants: required {required}, available {available}")]
    InsufficientParticipants { required: usize, available: usize },
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
}

impl From<sled::Error> for MpcError {
    fn from(err: sled::Error) -> Self {
        MpcError::StorageError(err.to_string())
    }
}

impl From<reqwest::Error> for MpcError {
    fn from(err: reqwest::Error) -> Self {
        MpcError::NetworkError(err.to_string())
    }
}

pub type MpcResult<T> = Result<T, MpcError>;
EOF

print_status 0 "MPC error handling created"

# Create environment files
echo "NODE_ID=1
BIND_ADDRESS=127.0.0.1:8001
DATA_DIR=./data/node1
PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
RUST_LOG=mpc=info,frost_ed25519=info" > mpc/.env.node1

echo "NODE_ID=2
BIND_ADDRESS=127.0.0.1:8002
DATA_DIR=./data/node2
PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
RUST_LOG=mpc=info,frost_ed25519=info" > mpc/.env.node2

echo "NODE_ID=3
BIND_ADDRESS=127.0.0.1:8003
DATA_DIR=./data/node3
PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
RUST_LOG=mpc=info,frost_ed25519=info" > mpc/.env.node3

print_status 0 "Environment files created"

echo ""
echo "Step 3: Updating Backend Configuration"
echo "-------------------------------------"

# Check if backend .env exists and update it
if [ -f "backend/.env" ]; then
    # Add MPC configuration to backend .env if not already present
    if ! grep -q "MPC_NODES" backend/.env; then
        echo "" >> backend/.env
        echo "# MPC Configuration" >> backend/.env
        echo "MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003" >> backend/.env
        echo "MPC_THRESHOLD=2" >> backend/.env
        print_status 0 "Backend .env updated with MPC configuration"
    else
        print_status 0 "Backend .env already has MPC configuration"
    fi
else
    echo -e "${YELLOW}⚠️  backend/.env not found. Please ensure it exists and add:${NC}"
    echo "MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003"
    echo "MPC_THRESHOLD=2"
fi

echo ""
echo "Step 4: Building Dependencies"
echo "----------------------------"

# Build MPC module
echo "Building MPC module (this may take a few minutes)..."
cd mpc
if cargo build --release; then
    print_status 0 "MPC module built successfully"
else
    echo -e "${YELLOW}⚠️  Release build failed, trying debug build...${NC}"
    if cargo build; then
        print_status 0 "MPC module built in debug mode"
    else
        print_status 1 "Failed to build MPC module"
    fi
fi
cd ..

echo ""
echo "Step 5: Creating Utility Scripts"
echo "-------------------------------"

# Make all scripts executable
chmod +x start_mpc_cluster.sh 2>/dev/null || true
chmod +x check_mpc_health.sh 2>/dev/null || true
chmod +x test_mpc_integration.sh 2>/dev/null || true
chmod +x test_phase3_complete.sh 2>/dev/null || true

print_status 0 "Scripts made executable"

echo ""
echo -e "${GREEN}🎉 Phase 3 Setup Complete!${NC}"
echo "========================="
echo ""
echo "Next Steps:"
echo "1. Copy the provided source files:"
echo "   - mpc/src/main.rs"
echo "   - mpc/src/tss.rs" 
echo "   - mpc/src/serialization.rs"
echo "   - backend/src/services/mpc.rs"
echo ""
echo "2. Start the MPC cluster:"
echo "   ./start_mpc_cluster.sh"
echo ""
echo "3. Verify cluster health:"
echo "   ./check_mpc_health.sh"
echo ""
echo "4. Test MPC integration:"
echo "   ./test_mpc_integration.sh"
echo ""
echo "5. Start the backend server:"
echo "   cd backend && cargo run"
echo ""
echo "6. Run complete integration test:"
echo "   ./test_phase3_complete.sh"
echo ""
echo -e "${YELLOW}Important Notes:${NC}"
echo "- MPC nodes store keys in ./mpc/data/node{1,2,3}/"
echo "- Each node runs on ports 8001, 8002, 8003"
echo "- Threshold is set to 2/3 (any 2 nodes can sign)"
echo "- This is a demo implementation - production needs security hardening"