# Test Suite Organization

This directory contains all test scripts for the MPC Solana Wallet project, organized by phase and functionality.

## 📁 Directory Structure

```
tests/
├── phase3/                     # Phase 3: Backend API Integration Tests
│   ├── integration/           # Integration test suites
│   │   ├── run_all.sh        # Main Phase 3 integration test runner
│   │   ├── test_complete.sh  # Complete Phase 3 validation
│   │   └── run_step_3_1.sh   # Step 3.1 specific tests
│   ├── auth/                  # Authentication & Security tests
│   │   ├── test_auth.sh      # Comprehensive auth tests
│   │   └── test_security.sh  # JWT and security validation
│   ├── wallet/                # Wallet operations tests
│   │   ├── test_flow.sh      # Complete wallet flow (keygen, sign, aggregate)
│   │   └── test_resilience.sh # Node failure and recovery tests
│   ├── api/                   # API layer tests
│   │   └── test_layer.sh     # CORS, rate limiting, OpenAPI tests
│   └── validation/            # Validation scripts
│       ├── validate.sh       # Online validation (requires running services)
│       └── validate_offline.sh # Offline compilation checks
├── phase4/                     # Phase 4: Solana Integration Tests
│   ├── test_complete.sh       # Complete Phase 4 validation
│   ├── test_solana_integration.sh # Solana blockchain integration
│   ├── test_solana_demo.sh    # Solana demo/examples
│   └── test_step5_complete.sh # Step 5 specific tests
├── mpc/                        # MPC Cluster Tests
│   ├── test_integration.sh    # MPC integration tests (basic functionality)
│   ├── test_cluster.sh        # MPC cluster health checks
│   ├── test_step2.sh          # Step 2 specific MPC tests
│   └── test_load.sh          # Load testing (50 concurrent users)
└── performance/                # Performance & Load Tests
    └── test_performance.sh    # API performance and latency tests
```

## 🚀 Quick Start

### Run All Phase 3 Tests
```bash
cd /path/to/purge-assignment
./tests/phase3/integration/run_all.sh
```

### Run All Phase 4 Tests
```bash
cd /path/to/purge-assignment
./tests/phase4/test_complete.sh
```

### Run MPC Tests
```bash
cd /path/to/purge-assignment
./tests/mpc/test_integration.sh
./tests/mpc/test_load.sh
```

## 📋 Test Categories

### Phase 3: Backend API Integration
**Purpose**: Validate complete backend API functionality with MPC integration

**Prerequisites**:
- PostgreSQL database running
- MPC cluster running (3 nodes on ports 8001-8003)
- Backend API running (port 8080)

**Tests Include**:
- ✅ Authentication & Security (JWT, user isolation)
- ✅ Wallet Operations (keygen, signing, aggregation)
- ✅ Resilience (node failures, recovery)
- ✅ API Layer (CORS, rate limiting, OpenAPI)
- ✅ Performance (concurrent users, latency)

**Run**:
```bash
# Complete integration test suite
./tests/phase3/integration/run_all.sh

# Individual tests
./tests/phase3/auth/test_auth.sh
./tests/phase3/wallet/test_flow.sh
./tests/phase3/api/test_layer.sh
```

### Phase 4: Solana Integration
**Purpose**: Validate Solana blockchain integration and transaction signing

**Prerequisites**:
- All Phase 3 prerequisites
- Solana RPC endpoint configured

**Tests Include**:
- ✅ Address derivation (hex to base58)
- ✅ Transaction building
- ✅ MPC-signed transactions
- ✅ Balance retrieval
- ✅ Jupiter DEX integration

**Run**:
```bash
./tests/phase4/test_complete.sh
./tests/phase4/test_solana_integration.sh
```

### MPC Cluster Tests
**Purpose**: Test MPC cluster functionality and threshold signing

**Prerequisites**:
- MPC cluster running (3 nodes)

**Tests Include**:
- ✅ Cluster startup and health checks
- ✅ Distributed key generation
- ✅ Threshold signing (2-of-3)
- ✅ Concurrent operations
- ✅ Load testing (50+ concurrent users)

**Run**:
```bash
# Integration tests
./tests/mpc/test_integration.sh

# Load testing (with debug mode)
DEBUG=1 ./tests/mpc/test_load.sh
```

### Performance Tests
**Purpose**: Validate system performance under load

**Tests Include**:
- ✅ Concurrent user load (5-50 users)
- ✅ Latency thresholds (< 5s response time)
- ✅ Memory usage validation
- ✅ Database connection pool testing

**Run**:
```bash
./tests/performance/test_performance.sh
```

## 🔍 Validation Scripts

### Online Validation
Tests that require running services:
```bash
./tests/phase3/validation/validate.sh
```

### Offline Validation
Compilation and syntax checks (no services required):
```bash
./tests/phase3/validation/validate_offline.sh
```

## 🎯 Test Execution Best Practices

### 1. Start Services First
```bash
# Start MPC cluster
./scripts/start_mpc_cluster.sh

# Start backend
cd backend && cargo run

# Verify health
curl http://localhost:8080/health
curl http://localhost:8001/health
```

### 2. Run Tests in Order
```bash
# 1. MPC tests first
./tests/mpc/test_integration.sh

# 2. Backend integration tests
./tests/phase3/integration/run_all.sh

# 3. Solana integration tests
./tests/phase4/test_solana_integration.sh

# 4. Load tests last
./tests/mpc/test_load.sh
./tests/performance/test_performance.sh
```

### 3. Debug Failed Tests
```bash
# Enable debug mode for detailed output
DEBUG=1 ./tests/mpc/test_load.sh

# Check service logs
tail -f backend/logs/*.log
tail -f mpc/data/node*/logs/*.log
```

## 📊 Success Criteria

### Phase 3 Tests
- All authentication tests pass (401 for unauthorized, 200 for valid JWT)
- Wallet flow completes successfully (keygen → sign → aggregate)
- Rate limiting enforced (429 after threshold)
- Resilience: System recovers from single node failure

### Phase 4 Tests
- Address derivation succeeds
- Transactions build and sign correctly
- Balance retrieval works
- Invalid inputs rejected properly

### MPC Tests
- Cluster starts with all 3 nodes healthy
- ≥80% success rate for concurrent operations
- Operations complete in < 5 seconds
- Load test: ≥95% success rate with 50 concurrent users

### Performance Tests
- ≥95% success rate under load
- Average response time < 5 seconds
- 95th percentile < 10 seconds
- Memory usage < 500MB

## 🔧 Troubleshooting

### Tests Hanging
- Check if services are running
- Verify network connectivity
- Check MPC cluster health
- Review service logs

### Authentication Failures
- Verify JWT_SECRET is set
- Check token expiration
- Ensure database is accessible

### MPC Failures
- Restart MPC cluster: `./scripts/stop_mpc_cluster.sh && ./scripts/start_mpc_cluster.sh`
- Check node logs: `tail -f mpc/data/node*/logs/*.log`
- Verify all 3 nodes are running: `ps aux | grep mpc`

### Database Issues
- Check connection: `psql $DATABASE_URL -c "\dt"`
- Run migrations: `./run_all_migrations.sh`
- Verify permissions: `./fix_db_permissions.sh`

## 📚 Additional Resources

- **Setup Guide**: `/docs/setup-index.md`
- **Test Script Fixes**: `/docs/test-scripts-fixes.md`
- **Phase 3 Documentation**: `/docs/phase3-completion-summary.md`
- **Phase 4 Documentation**: `/docs/phase4-step1-solana-integration.md`

## 🗺️ Migration Mapping

Old test locations → New test locations:

```
test_phase3_complete.sh          → tests/phase3/integration/test_complete.sh
phase3_integration_tests.sh      → tests/phase3/integration/run_all.sh
run_step_3_1_tests.sh            → tests/phase3/integration/run_step_3_1.sh
test_phase3_auth.sh              → tests/phase3/auth/test_auth.sh
test_auth_security.sh            → tests/phase3/auth/test_security.sh
test_wallet_flow.sh              → tests/phase3/wallet/test_flow.sh
test_resilience.sh               → tests/phase3/wallet/test_resilience.sh
test_api_layer.sh                → tests/phase3/api/test_layer.sh
test_performance.sh              → tests/performance/test_performance.sh
validate_phase3.sh               → tests/phase3/validation/validate.sh
validate_phase3_offline.sh       → tests/phase3/validation/validate_offline.sh
test_phase4.sh                   → tests/phase4/test_complete.sh
test_solana_integration.sh       → tests/phase4/test_solana_integration.sh
test_solana_demo.sh              → tests/phase4/test_solana_demo.sh
test_step5_complete.sh           → tests/phase4/test_step5_complete.sh
test_mpc_integration.sh          → tests/mpc/test_integration.sh
test_mpc_cluster.sh              → tests/mpc/test_cluster.sh
test_mpc_step2.sh                → tests/mpc/test_step2.sh
test_mpc_load.sh                 → tests/mpc/test_load.sh
```

---

**Last Updated**: December 2024
**Status**: ✅ All tests organized and documented 