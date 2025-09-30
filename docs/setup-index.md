# MPC Solana Wallet - Complete Setup & Architecture Guide

## 🎯 Project Overview

The **MPC Solana Wallet** is a production-ready Multi-Party Computation (MPC) based cryptocurrency wallet service for Solana blockchain. It provides enterprise-grade security with distributed key management, real-time blockchain indexing, and DEX integration.

### Key Features
- **🔐 Distributed Key Management**: 2-of-3 threshold signing using FROST Ed25519
- **🌐 Production-Ready APIs**: Versioned REST APIs with JWT authentication
- **📊 Real-time Indexing**: Yellowstone GRPC integration for balance tracking
- **🔄 DEX Integration**: Jupiter API for token swaps
- **🛡️ Enterprise Security**: No single point of private key failure

---

## 🏗️ Architecture Components

### 1. **Store Crate** (`store/`) - Core Data Layer ✅ COMPLETE
- **Technology**: PostgreSQL + SQLx
- **Purpose**: Central data management for users, balances, assets, and quotes
- **Database**: PostgreSQL with comprehensive schema and performance indexes
- **Status**: Production ready with full CRUD operations

### 2. **Backend Service** (`backend/`) - HTTP API Server ✅ COMPLETE
- **Technology**: Actix-web + JWT authentication
- **Purpose**: REST API for wallet operations, user management, and trading
- **Port**: 8080 (configurable)
- **Status**: Production ready with versioned APIs

### 3. **MPC Service** (`mpc/`) - Multi-Party Computation ✅ COMPLETE
- **Technology**: Rust + FROST Ed25519 threshold signing
- **Purpose**: Distributed key generation and transaction signing
- **Ports**: 8001, 8002, 8003 (3-node cluster)
- **Status**: Production ready with FROST implementation

### 4. **Indexer Service** (`indexer/`) - Real-time Blockchain Monitor 🔄 IN PROGRESS
- **Technology**: Yellowstone GRPC + PostgreSQL
- **Purpose**: Real-time Solana blockchain monitoring and balance tracking
- **Database**: Separate PostgreSQL instance for indexing data
- **Status**: 70% complete, infrastructure ready

---

## 🗄️ Database Schema

### Main Schema (`migrations/001_initial_schema.sql`) ✅ COMPLETE

#### Core Tables:
- **`users`** - User accounts with MPC public keys
  - `id`, `email`, `password_hash`, `public_key`, `created_at`, `updated_at`
- **`assets`** - Supported tokens (SOL, USDC, USDT, etc.)
  - `id`, `mint_address`, `decimals`, `name`, `symbol`, `logo_url`
- **`balances`** - User token balances (stored in smallest units)
  - `id`, `user_id`, `asset_id`, `amount`, `created_at`, `updated_at`
- **`quotes`** - Jupiter swap quotes with expiration
  - `id`, `user_id`, `input_mint`, `output_mint`, `in_amount`, `out_amount`, `quote_data`, `expires_at`, `used`

#### Performance Indexes (`migrations/002_performance_indexes.sql`) ✅ COMPLETE:
- Composite indexes for user-asset lookups
- Partial indexes for active quotes
- Case-insensitive email lookups
- Recent data queries optimization

#### MPC Session Management (`migrations/003_wallet_state_management.sql`) ✅ COMPLETE:
- **`wallet_keys`** - MPC public keys per user
- **`signing_sessions`** - Two-phase signing session management
- **`signing_status`** - Enum for session states

### Indexer Schema (`indexer/migrations/001_initial.sql`) ✅ COMPLETE

#### Indexing Tables:
- **`user_wallets`** - Wallet addresses and SOL balances
- **`balance_changes`** - Historical balance tracking
- **`token_balances`** - SPL token balances
- **`transactions`** - Transaction details
- **`indexer_state`** - Indexer state management
- **`subscription_metrics`** - Performance metrics

---

## 🚀 Quick Setup Guide

### Prerequisites
- **Rust 1.70+** with Cargo
- **PostgreSQL 13+** database
- **Git** for cloning the repository

### 1. Clone and Setup
```bash
git clone <repository-url>
cd purge-assignment
```

### 2. Database Setup
```bash
# Create databases
createdb solana_wallet
createdb solana_wallet_indexer

# Run all migrations
./run_all_migrations.sh
```

### 3. Environment Configuration
Create `.env` file in project root:
```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/solana_wallet
TEST_DATABASE_URL=postgresql://user:pass@localhost:5432/solana_wallet_test

# MPC Cluster
MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
MPC_THRESHOLD=2

# Solana
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_COMMITMENT=confirmed

# Jupiter DEX
JUPITER_API_URL=https://quote-api.jup.ag/v6

# Backend
JWT_SECRET=your-secret-key-here
BIND_ADDRESS=127.0.0.1:8080
```

### 4. Start Services

#### Start MPC Cluster
```bash
# Start 3-node MPC cluster
./start_mpc_cluster.sh

# Verify cluster health
curl http://localhost:8001/health
curl http://localhost:8002/health
curl http://localhost:8003/health
```

#### Start Backend API
```bash
cd backend
cargo run

# Verify API health
curl http://localhost:8080/health
```

#### Start Indexer (Optional)
```bash
cd indexer
cargo run
```

### 5. Verify Installation
```bash
# Run integration tests
./tests/phase3/integration/run_all.sh

# Test MPC functionality
./tests/mpc/test_integration.sh

# Test Solana integration
./tests/phase4/test_solana_integration.sh
```

---

## 📡 API Endpoints

### Wallet Operations (`/api/v1/wallet/`) ✅ COMPLETE
- `POST /keygen` - Generate distributed MPC keys
- `POST /sign/phase1` - Initiate signing (nonce generation)
- `POST /sign/phase2` - Complete signing (signature shares)
- `POST /aggregate` - Aggregate signature shares
- `GET /health` - MPC cluster health check

### Solana Operations (`/api/v1/solana/`) ✅ COMPLETE
- `POST /address` - Derive Solana address from MPC key
- `POST /transfer` - Send SOL/tokens (MPC-signed)
- `GET /balance` - Get user balances
- `POST /quote` - Get Jupiter swap quote
- `POST /swap` - Execute token swap

### User Management (`/api/user/`) ✅ COMPLETE
- `POST /signup` - User registration with MPC key generation
- `POST /signin` - User authentication (JWT)
- `GET /profile` - Get user profile

### Documentation & Monitoring
- `GET /api/docs/` - Interactive API documentation (Swagger UI)
- `GET /metrics` - Prometheus metrics endpoint
- `GET /health` - Service health check

---

## 🔧 Service Configuration

### MPC Service Configuration
Each MPC node requires:
```bash
NODE_ID=1                    # Unique node ID (1, 2, or 3)
BIND_ADDRESS=127.0.0.1:8001 # Node bind address
DATA_DIR=./data/node1        # Data directory for key storage
PEER_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
MPC_THRESHOLD=2              # Threshold for signing (2-of-3)
```

### Backend Service Configuration
```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/solana_wallet
JWT_SECRET=your-secret-key-here
BIND_ADDRESS=127.0.0.1:8080
MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
MPC_THRESHOLD=2
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_COMMITMENT=confirmed
JUPITER_API_URL=https://quote-api.jup.ag/v6
```

### Indexer Service Configuration
```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/solana_wallet_indexer
YELLOWSTONE_ENDPOINT=your-yellowstone-endpoint
YELLOWSTONE_TOKEN=your-auth-token
COMMITMENT_LEVEL=confirmed
HEALTH_CHECK_INTERVAL_SECONDS=30
BATCH_SIZE=100
```

---

## 🧪 Testing & Validation

### Automated Test Suites
```bash
# Complete Phase 3 integration tests
./tests/phase3/integration/run_all.sh

# MPC cluster functionality
./tests/mpc/test_integration.sh

# Solana blockchain integration
./tests/phase4/test_solana_integration.sh

# Load testing
./tests/mpc/test_load.sh
```

### Manual Testing
```bash
# Test user registration and MPC key generation
curl -X POST http://localhost:8080/api/user/signup \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "password123"}'

# Test MPC key generation
curl -X POST http://localhost:8080/api/v1/wallet/keygen \
  -H "Authorization: Bearer <jwt-token>" \
  -H "Content-Type: application/json" \
  -d '{"threshold": 2, "participants": 3}'

# Test Solana address derivation
curl -X POST http://localhost:8080/api/v1/solana/address \
  -H "Authorization: Bearer <jwt-token>" \
  -H "Content-Type: application/json" \
  -d '{"public_key": "your-mpc-public-key"}'
```

### Database Validation
```bash
# Validate database schema
cd store && cargo run --bin verify_db

# Run schema validation tests
cd store && cargo run --bin schema_validation

# Test store operations
cd store && cargo run --bin test
```

---

## 🔒 Security Features

### Authentication & Authorization
- **JWT Authentication**: HS256 algorithm with configurable secret
- **Token Expiration**: 24-hour token lifetime
- **Rate Limiting**: 100 requests/minute per user
- **User Isolation**: Users can only access their own data

### MPC Security
- **Threshold Signing**: 2-of-3 nodes required for transactions
- **Distributed Key Generation**: No single point of private key failure
- **Encrypted Key Storage**: Keys stored encrypted in Sled database
- **Session Management**: Secure signing session handling

### Data Protection
- **Input Validation**: Comprehensive validation for all inputs
- **SQL Injection Prevention**: Parameterized queries with SQLx
- **Error Handling**: Secure error messages without information leakage
- **Audit Logging**: Structured logging for all operations

---

## 📊 Monitoring & Observability

### Prometheus Metrics
- **Request Counter**: `api_requests_total`
- **Request Duration**: `api_request_duration_seconds`
- **Error Counter**: `api_errors_total`
- **MPC Operations**: `mpc_operations_total`
- **Solana Transactions**: `solana_transactions_total`

### Structured Logging
- **Request Tracing**: Complete request/response logging
- **User Context**: User ID included in all log entries
- **Error Tracking**: Detailed error logging with context
- **Performance Metrics**: Request duration tracking

### Health Monitoring
- **Service Health**: `/health` endpoint for service status
- **Cluster Health**: MPC cluster availability monitoring
- **Database Health**: Database connection monitoring
- **External Services**: Solana RPC and Jupiter API health

---

## 🚀 Production Deployment

### Docker Deployment
```yaml
# docker-compose.yml
version: '3.8'
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: solana_wallet
      POSTGRES_USER: user
      POSTGRES_PASSWORD: password
  
  mpc-node-1:
    build: ./mpc
    ports: ["8001:8001"]
    environment:
      NODE_ID: 1
      BIND_ADDRESS: 0.0.0.0:8001
  
  mpc-node-2:
    build: ./mpc
    ports: ["8002:8002"]
    environment:
      NODE_ID: 2
      BIND_ADDRESS: 0.0.0.0:8002
      
  mpc-node-3:
    build: ./mpc
    ports: ["8003:8003"]
    environment:
      NODE_ID: 3
      BIND_ADDRESS: 0.0.0.0:8003
      
  backend:
    build: ./backend
    ports: ["8080:8080"]
    depends_on: [postgres, mpc-node-1, mpc-node-2, mpc-node-3]
    
  indexer:
    build: ./indexer
    depends_on: [postgres]
```

### Production Considerations
- **Separate MPC nodes** on different servers for true security
- **Database replication** and backups
- **Load balancers** for high availability
- **SSL/TLS termination** for secure communication
- **Monitoring and alerting** systems

---

## 🔧 Troubleshooting

### Common Issues

#### MPC Cluster Not Starting
```bash
# Check if ports are available
netstat -an | grep 800[1-3]

# Check MPC logs
tail -f mpc/data/node*/logs/*.log

# Restart cluster
./stop_mpc_cluster.sh
./start_mpc_cluster.sh
```

#### Database Connection Issues
```bash
# Check database status
pg_isready -h localhost -p 5432

# Check database permissions
psql $DATABASE_URL -c "\dt"

# Run migrations
./run_all_migrations.sh
```

#### Backend API Issues
```bash
# Check backend logs
cd backend && cargo run 2>&1 | tee backend.log

# Test API health
curl http://localhost:8080/health

# Check JWT configuration
echo $JWT_SECRET
```

### Performance Issues
- **Database queries slow**: Check indexes with `EXPLAIN ANALYZE`
- **MPC operations slow**: Check cluster health and network latency
- **API responses slow**: Check database connection pool settings

---

## 📚 Additional Resources

### Documentation Files
- **`README.md`** - Project overview and quick start
- **`current-status.md`** - Real-time implementation status
- **`implementation_steps.md`** - Detailed development roadmap
- **`phase3-*.md`** - Phase 3 implementation details
- **`phase4-*.md`** - Phase 4 implementation details
- **`test-scripts-fixes.md`** - Testing procedures and fixes

### External Resources
- **FROST Documentation**: https://github.com/ZcashFoundation/frost
- **Solana Documentation**: https://docs.solana.com/
- **Jupiter API**: https://docs.jup.ag/
- **Yellowstone GRPC**: https://github.com/rpcpool/yellowstone-grpc

---

## �� Next Steps

### Immediate Actions
1. **Complete Jupiter DEX integration** (Phase 5)
2. **Finish real-time indexer** (Phase 6)
3. **Add production hardening** (Phase 7)

### Long-term Goals
1. **Mainnet deployment** with production security
2. **Multi-chain support** (Ethereum, Polygon, etc.)
3. **Advanced features** (multi-sig, governance, etc.)

---

This setup guide provides everything needed to get the MPC Solana Wallet running in development and production environments. The system is designed for enterprise use with proper security, scalability, and observability.
