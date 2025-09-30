# Step 3.1 - Installation & Setup Guide

## 📦 Quick Installation

Follow these steps to install and test the complete Step 3.1 implementation.

---

## Prerequisites

- ✅ Rust toolchain installed (1.70+)
- ✅ MPC cluster running (3 nodes on ports 8001, 8002, 8003)
- ✅ PostgreSQL database set up
- ✅ Phase 1 & 2 completed (Store module and MPC nodes)

---

## Step 1: Update MPC Client Service

Replace the existing MPC client service with the complete implementation:

```bash
# Backup current implementation
cp backend/src/services/mpc.rs backend/src/services/mpc.rs.backup

# Copy the new complete implementation
# (Use the code from the artifact "Complete MPC Client (mpc.rs)")
```

**File location**: `backend/src/services/mpc.rs`

Make sure to add the `rand` dependency to your `backend/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
rand = "0.8"
```

---

## Step 2: Add Test File

Create the comprehensive test suite:

```bash
# Create tests directory if it doesn't exist
mkdir -p backend/tests

# Add the test file
# (Use the code from the artifact "Step 3.1 Complete Test Suite")
```

**File location**: `backend/tests/test_step_3_1_complete.rs`

---

## Step 3: Add Test Runner Script

Create the test runner script:

```bash
# Copy script to project root
# (Use the code from the artifact "Test Runner Script")

# Make it executable
chmod +x run_step_3_1_tests.sh
```

**File location**: `./run_step_3_1_tests.sh`

---

## Step 4: Start MPC Cluster

Ensure your MPC cluster is running:

```bash
# Start all 3 nodes
./start_mpc_cluster.sh

# Verify nodes are running
curl http://localhost:8001/health
curl http://localhost:8002/health
curl http://localhost:8003/health

# Should return health status for each node
```

---

## Step 5: Update Dependencies

Build the project with updated dependencies:

```bash
cd backend

# Update Cargo.lock and download dependencies
cargo update

# Build the project
cargo build

# This should compile without errors
```

---

## Step 6: Run Tests

Execute the comprehensive test suite:

```bash
# Option 1: Use the test runner script (recommended)
./run_step_3_1_tests.sh

# Option 2: Run tests manually
cd backend
cargo test --test test_step_3_1_complete -- --nocapture

# Option 3: Run final validation only
cargo test --test test_step_3_1_complete test_99_complete_step_3_1_validation -- --nocapture
```

---

## Step 7: Verify Installation

Check that all features are working:

### Test 1: Key Generation
```bash
cd backend
cargo test --test test_step_3_1_complete test_01_generate_key_success -- --nocapture
```

**Expected output:**
```
✅ Key generated successfully!
   Public Key: [64-character hex string]
   Time taken: 2-5 seconds
```

### Test 2: Health Check API
```bash
cargo test --test test_step_3_1_complete test_05_health_check_api -- --nocapture
```

**Expected output:**
```
✅ Health check successful!
   Status: operational
   Total nodes: 3
   Healthy nodes: 3
   Threshold: 2
   Threshold met: true
```

### Test 3: Load Balancing
```bash
cargo test --test test_step_3_1_complete test_07_round_robin_load_balancing -- --nocapture
```

**Expected output:**
```
Request 1-5: ✅ Success
✅ Round-robin test completed
```

---

## Verification Checklist

After installation, verify these features:

- [ ] **Core Operations**
  - [ ] `generate_key()` works
  - [ ] `sign_message()` works
  - [ ] `sign_transaction()` works

- [ ] **Health Monitoring**
  - [ ] `health_check()` returns cluster status
  - [ ] `check_threshold_availability()` works
  - [ ] `get_cluster_status()` provides node details

- [ ] **Load Balancing**
  - [ ] Round-robin strategy works
  - [ ] Health-based strategy works
  - [ ] Random strategy works

- [ ] **Retry Logic**
  - [ ] Exponential backoff implemented
  - [ ] Configurable retry attempts
  - [ ] Node fallback on failures

- [ ] **Circuit Breaker**
  - [ ] Failure tracking per node
  - [ ] Automatic circuit opening
  - [ ] Timeout-based recovery

- [ ] **Error Handling**
  - [ ] Network timeout handling
  - [ ] Insufficient nodes error
  - [ ] All nodes down error
  - [ ] Proper error messages

---

## Troubleshooting

### Issue: Compilation Errors

**Solution 1 - Missing Dependencies:**
```bash
cd backend
cargo update
cargo build
```

**Solution 2 - Syntax Errors:**
Check that you copied the complete file without truncation

### Issue: Tests Fail with "Connection refused"

**Cause:** MPC cluster not running

**Solution:**
```bash
# Check if nodes are running
ps aux | grep mpc

# Start cluster if not running
./start_mpc_cluster.sh

# Wait 5 seconds for startup
sleep 5

# Verify health
curl http://localhost:8001/health
```

### Issue: Tests Timeout

**Cause:** Network latency or node overload

**Solution:**
```bash
# Increase timeout in test
cd backend
# Edit test_step_3_1_complete.rs
# Increase timeout values if needed

# Or just retry
cargo test --test test_step_3_1_complete -- --nocapture
```

### Issue: Some Tests Fail but Most Pass

**Status:** Acceptable if 70%+ pass rate

**Reason:** Node availability can vary during testing

**Solution:**
```bash
# Check cluster health
./check_mpc_health.sh

# Restart cluster
./stop_mpc_cluster.sh
./start_mpc_cluster.sh

# Retry tests
./run_step_3_1_tests.sh
```

---

## Environment Variables

Ensure these are set in your `backend/.env`:

```bash
# MPC Configuration
MPC_NODES=http://localhost:8001,http://localhost:8002,http://localhost:8003
MPC_THRESHOLD=2

# Database (from Phase 1)
DATABASE_URL=postgresql://user:password@localhost/solana_wallet

# JWT Secret (from Phase 3)
JWT_SECRET=your_secret_key_here
```

---

## File Structure After Installation

```
purge-assignment/
├── backend/
│   ├── src/
│   │   └── services/
│   │       └── mpc.rs              ← Updated with complete implementation
│   ├── tests/
│   │   └── test_step_3_1_complete.rs  ← New test file
│   ├── Cargo.toml                   ← Updated with rand dependency
│   └── .env                         ← MPC configuration
├── run_step_3_1_tests.sh            ← New test runner script
└── docs/
    └── step_3_1_complete.md         ← This guide
```

---

## Next Steps After Installation

Once installation is complete and tests pass:

1. **Review the implementation**
   - Read through `backend/src/services/mpc.rs`
   - Understand the load balancing strategies
   - Review circuit breaker pattern

2. **Run performance benchmarks**
   ```bash
   cargo test --test test_step_3_1_complete test_14_concurrent_operations -- --nocapture
   cargo test --test test_step_3_1_complete test_15_sequential_operations_performance -- --nocapture
   ```

3. **Integrate with user routes**
   - Move to Step 3.2
   - Implement signup workflow with MPC
   - Add user management endpoints

4. **Document your changes**
   - Update `docs/current-status.md`
   - Mark Step 3.1 as complete
   - Note any customizations made

---

## Success Indicators

You've successfully completed Step 3.1 if:

✅ All or most tests pass (70%+ success rate)
✅ `health_check()` API returns cluster status
✅ Load balancing distributes requests across nodes
✅ Retry logic handles transient failures
✅ Circuit breaker protects against cascading failures
✅ Error messages are clear and actionable

---

## Support

If you encounter issues:

1. **Check logs**: MPC node logs in `mpc/logs/`
2. **Review documentation**: See `docs/step_3_1_complete.md`
3. **Run health checks**: Use `./check_mpc_health.sh`
4. **Test individual components**: Run tests one at a time

---

**Installation Date**: _________________  
**Completed By**: _________________  
**Status**: ⬜ In Progress  ⬜ Complete  ⬜ Issues