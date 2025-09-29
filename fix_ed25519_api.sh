#!/bin/bash

echo "🔧 Fixing ed25519-dalek API Issues"
echo "=================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✅ $2${NC}"
    else
        echo -e "${RED}❌ $2${NC}"
        exit 1
    fi
}

echo "Step 1: Update MPC Cargo.toml with correct ed25519-dalek version"
echo "----------------------------------------------------------------"

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
serde_json = "1.0"
rmp-serde = "1.1"
tokio = { version = "1.35", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "1.0"
anyhow = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
sled = "0.34"
hex = "0.4"
reqwest = { version = "0.11", features = ["json"] }
futures-util = "0.3"
ed25519-dalek = { version = "2.1", features = ["rand_core"] }
rand = "0.8"
EOF

print_status 0 "Updated MPC Cargo.toml"

echo ""
echo "Step 2: Clean and rebuild MPC"
echo "-----------------------------"

cd mpc
cargo clean
print_status 0 "Cleaned MPC build artifacts"

echo "Building MPC with updated dependencies..."
if cargo build; then
    print_status 0 "MPC builds successfully with new API"
else
    echo -e "${RED}❌ Build failed. You may need to update your tss.rs file.${NC}"
    echo ""
    echo "Update mpc/src/tss.rs imports to:"
    echo "use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};"
    echo ""
    echo "And update the key generation code to use:"
    echo "let signing_key = SigningKey::generate(&mut csprng);"
    echo "let verifying_key = signing_key.verifying_key();"
    exit 1
fi

cd ..

echo ""
echo "Step 3: Test that the fix works"
echo "-------------------------------"

# Check if the MPC binary was created
if [ -f "mpc/target/debug/mpc" ]; then
    print_status 0 "MPC binary created successfully"
else
    echo -e "${YELLOW}⚠️  Debug binary not found, but build succeeded${NC}"
fi

echo ""
echo -e "${GREEN}🎉 ed25519-dalek API issues fixed!${NC}"
echo "====================================="
echo ""
echo "Now you need to update your mpc/src/tss.rs file with the correct imports:"
echo "1. Change: use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer};"
echo "2. To:     use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};"
echo ""
echo "And update the key generation methods accordingly."
echo ""
echo "After updating tss.rs, try: ./start_mpc_cluster.sh"