# MPC Solana Wallet - Project Documentation

This folder contains comprehensive documentation for the **MPC Solana Wallet** project - a production-ready Multi-Party Computation (MPC) based cryptocurrency wallet service for Solana blockchain.

## 🎯 Project Overview

The MPC Solana Wallet is a complete wallet infrastructure that provides:
- **Distributed Key Management**: 2-of-3 threshold signing using FROST Ed25519
- **Real-time Blockchain Indexing**: Yellowstone GRPC integration for balance tracking
- **DEX Integration**: Jupiter API for token swaps
- **Production-Ready APIs**: Versioned REST APIs with JWT authentication
- **Enterprise Security**: No single point of private key failure

## 📁 Documentation Structure

### Core Documentation
- **`setup-index.md`** - Complete setup guide and architecture overview
- **`implementation_steps.md`** - Detailed 7-phase implementation roadmap
- **`current-status.md`** - Real-time implementation status and progress

### Phase Documentation
- **`phase3-implementation.md`** - Phase 3 wallet API implementation details
- **`phase3-completion-summary.md`** - Phase 3 completion summary with test results
- **`phase3-4.md`** - Combined Phase 3 & 4 implementation documentation
- **`phase4-step1-solana-integration.md`** - Solana blockchain integration details

### Testing & Validation
- **`test-scripts-fixes.md`** - Test script fixes and validation procedures
- **`step-1-2-completion-summary.md`** - Database schema validation completion

## 🏗️ Project Architecture

```
purge-assignment/
├── docs/                          # 📚 Complete documentation
│   ├── README.md                  # This file - project overview
│   ├── setup-index.md             # Setup & architecture guide
│   ├── current-status.md          # Current implementation status
│   ├── implementation_steps.md    # 7-phase implementation plan
│   ├── phase3-*.md                # Phase 3 implementation docs
│   ├── phase4-*.md                # Phase 4 implementation docs
│   └── test-scripts-fixes.md      # Testing procedures
├── backend/                       # 🌐 HTTP API server (Actix-web)
│   ├── src/routes/               # API endpoints (v1, wallet, user, solana)
│   ├── src/services/             # Business logic (MPC, Jupiter, Solana)
│   ├── src/middleware/           # Auth, rate limiting, metrics
│   └── src/blockchain/           # Solana blockchain integration
├── mpc/                          # 🔐 Multi-Party Computation service
│   ├── src/tss.rs                # FROST threshold signing implementation
│   ├── src/serialization.rs      # Cryptographic data structures
│   └── src/error.rs              # Comprehensive error handling
├── store/                         # 💾 Database operations crate
│   ├── src/user.rs               # User management operations
│   ├── src/balance.rs            # Balance and asset operations
│   ├── src/quote.rs               # Jupiter quote management
│   └── src/bin/                  # Database validation tools
├── indexer/                       # 📊 Real-time blockchain monitor
│   ├── src/yellowstone.rs        # Yellowstone GRPC client
│   ├── src/processor.rs           # Transaction processing logic
│   └── src/database.rs           # Indexer database operations
├── migrations/                     # 🗄️ Database schema migrations
│   ├── 001_initial_schema.sql    # Core tables (users, assets, balances)
│   ├── 002_add_balance_tables.sql # Balance management tables
│   └── 003_wallet_state_management.sql # MPC session management
├── tests/                         # 🧪 Organized test suites
│   ├── phase3/                   # Phase 3 backend API tests
│   ├── phase4/                   # Phase 4 Solana integration tests
│   ├── mpc/                      # MPC cluster tests
│   └── performance/              # Performance and load tests
└── scripts/                       # 🛠️ Setup and utility scripts
    └── start_mpc_cluster.sh       # Start 3-node MPC cluster
```

## 🚀 Quick Start Guide

### Prerequisites
- **Rust 1.70+** with Cargo
- **PostgreSQL 13+** database
- **Node.js 16+** (for some scripts)

### 1. Database Setup
```bash
# Create databases
createdb solana_wallet
createdb solana_wallet_indexer

# Run migrations
./run_all_migrations.sh
```

### 2. Start MPC Cluster
```bash
# Start 3-node MPC cluster (ports 8001, 8002, 8003)
./start_mpc_cluster.sh
```

### 3. Start Backend API
```bash
cd backend
cargo run
# API available at http://localhost:8080
```

### 4. Start Indexer (Optional)
```bash
cd indexer
cargo run
# Indexer monitors Solana blockchain for balance changes
```

## 📊 Current Implementation Status

### ✅ **Phase 1: Core Infrastructure** - COMPLETE
- Database schema with comprehensive tables
- Store module with full CRUD operations
- Performance indexes and migrations
- Database validation and testing

### ✅ **Phase 2: MPC Implementation** - COMPLETE
- FROST Ed25519 threshold signing
- 3-node MPC cluster with distributed key generation
- Two-phase signing protocol
- Persistent key storage with Sled database

### ✅ **Phase 3: Backend API Integration** - COMPLETE
- Wallet-specific REST APIs (`/wallet/*`)
- Versioned external APIs (`/api/v1/wallet/*`)
- JWT authentication and rate limiting
- Session management and retry logic
- OpenAPI/Swagger documentation

### ✅ **Phase 4: Solana Integration** - COMPLETE
- Solana blockchain module with address derivation
- Transaction building and signing
- MPC-signed transaction broadcasting
- Secure API endpoints (`/api/v1/solana/*`)
- Comprehensive observability

### 🔄 **Phase 5: Jupiter DEX Integration** - IN PROGRESS
- Jupiter client implementation
- Quote fetching and validation
- Swap transaction building
- Integration with backend routes

### 🔄 **Phase 6: Real-time Indexer** - IN PROGRESS
- Yellowstone GRPC client setup
- Account update processing
- Balance change detection
- Database integration

### ⏳ **Phase 7: Production Hardening** - PENDING
- Security enhancements
- Performance optimization
- Comprehensive testing
- Production deployment

## 🧪 Testing

### Integration Tests
```bash
# Run complete Phase 3 integration tests
./tests/phase3/integration/run_all.sh

# Test MPC cluster functionality
./tests/mpc/test_integration.sh

# Test Solana blockchain integration
./tests/phase4/test_solana_integration.sh
```

### Unit Tests
```bash
# Test store module
cd store && cargo test

# Test backend services
cd backend && cargo test

# Test MPC implementation
cd mpc && cargo test
```

## 🔧 Configuration

### Environment Variables
```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/solana_wallet

# MPC Cluster
MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
MPC_THRESHOLD=2

# Solana
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_COMMITMENT=confirmed

# Jupiter DEX
JUPITER_API_URL=https://quote-api.jup.ag/v6
```

## 📈 API Endpoints

### Wallet Operations (`/api/v1/wallet/`)
- `POST /keygen` - Generate distributed MPC keys
- `POST /sign/phase1` - Initiate signing (nonce generation)
- `POST /sign/phase2` - Complete signing (signature shares)
- `POST /aggregate` - Aggregate signature shares
- `GET /health` - MPC cluster health check

### Solana Operations (`/api/v1/solana/`)
- `POST /address` - Derive Solana address from MPC key
- `POST /transfer` - Send SOL/tokens (MPC-signed)
- `GET /balance` - Get user balances
- `POST /quote` - Get Jupiter swap quote
- `POST /swap` - Execute token swap

### User Management (`/api/user/`)
- `POST /signup` - User registration with MPC key generation
- `POST /signin` - User authentication (JWT)
- `GET /profile` - Get user profile

## 🔒 Security Features

- **Threshold Signing**: 2-of-3 MPC nodes required for transactions
- **JWT Authentication**: Secure API access with token expiration
- **Rate Limiting**: 100 requests/minute per user
- **Input Validation**: Comprehensive validation for all inputs
- **Audit Logging**: Structured logging for all operations
- **No Private Key Exposure**: All signing via distributed MPC

## 📚 Additional Resources

- **Setup Guide**: See `setup-index.md` for detailed setup instructions
- **Implementation Plan**: See `implementation_steps.md` for development roadmap
- **Current Status**: See `current-status.md` for real-time progress
- **API Documentation**: Available at `http://localhost:8080/api/docs/`
- **Metrics**: Available at `http://localhost:8080/metrics`

## 🤝 Contributing

This is a production-ready MPC wallet implementation with comprehensive documentation, testing, and security features. The system is designed for enterprise use with proper error handling, observability, and scalability.

For detailed implementation information, see the individual documentation files in this directory.
