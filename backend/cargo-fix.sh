#!/bin/bash

echo "🔧 Fixing Cargo.toml configuration..."

cd /Users/vansh/Coding/SuperDevs/Assignments/purge-assignment/backend

# Check the current Cargo.toml
echo "📋 Current backend/Cargo.toml:"
cat Cargo.toml
echo ""

# Create a proper binary-only Cargo.toml
echo "📝 Creating fixed Cargo.toml..."
cat > Cargo.toml << 'EOF'
[package]
name = "backend"
version = "0.1.0"
edition = "2021"

# Specify this is a binary package only
[[bin]]
name = "backend"
path = "src/main.rs"

[dependencies]
# Web framework
actix-web = "4.11.0"

# Async runtime
tokio = { version = "1.47.1", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid", "migrate"] }

# Authentication
jsonwebtoken = "9"
bcrypt = "0.15"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# Logging
tracing = "0.1"
env_logger = "0.11"

# Environment
dotenv = "0.15"

# UUID
uuid = { version = "1.0", features = ["v4", "serde"] }

# HTTP client (for MPC communication)
reqwest = { version = "0.11", features = ["json"] }
EOF

echo "✅ Fixed Cargo.toml configuration"

# Now test the build
echo "🧪 Testing build..."
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_wallet"

if cargo build; then
    echo "🎉 Backend builds successfully!"
    echo ""
    echo "🚀 You can now run:"
    echo "export DATABASE_URL=\"postgresql://postgres:postgres@localhost:5432/solana_wallet\""
    echo "cargo run"
else
    echo "❌ Build still has issues. Let's check what's wrong..."
    echo ""
    echo "📋 Let's check your project structure:"
    echo "Current directory contents:"
    find . -name "*.rs" -o -name "Cargo.toml" | head -20
fi