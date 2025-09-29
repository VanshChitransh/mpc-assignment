# MPC Solana Wallet - Complete Implementation Plan

## Project Overview
Building a Multi-Party Computation (MPC) based crypto wallet service for Solana - essentially a mini-Fireblocks. Users don't control private keys directly; instead, keys are distributed across multiple MPC servers for enhanced security.

## Current Status Assessment

### ✅ Completed Components
- [x] Workspace structure (backend, indexer, mpc, store)
- [x] Database schema and migrations
- [x] Basic store module structure
- [x] JWT authentication framework
- [x] Basic route structure
- [x] Environment configuration setup
- [x] Docker/deployment scripts foundation

### ⚠️ Partially Implemented
- [ ] MPC key generation and signing (setup exists, needs completion)
- [ ] Store module CRUD operations (basic structure, needs full implementation)
- [ ] Backend API endpoints (routes exist, need MPC integration)
- [ ] Jupiter integration (client setup, needs transaction flow)
- [ ] Indexer (basic Yellowstone setup, needs processing logic)

### ❌ Missing Critical Components
- [ ] End-to-end transaction signing with MPC
- [ ] Real-time balance indexing and updates
- [ ] Complete Jupiter swap workflow
- [ ] Error handling and recovery mechanisms
- [ ] Production security measures

---

## Implementation Plan - 7 Phases

### **Phase 1: Core Infrastructure Completion (Days 1-2)**
*Foundation layer - database operations and basic services*

#### Step 1.1: Complete Store Module Implementation
**Priority: CRITICAL** - All services depend on this

**Files to modify:**
- `store/src/user.rs` - Complete user CRUD operations
- `store/src/balance.rs` - Balance management with asset discovery
- `store/src/quote.rs` - Jupiter quote storage
- `store/src/lib.rs` - Main store interface

**Tasks:**
1. Implement complete user operations:
   ```rust
   - create_user(email, password) -> User
   - verify_user_password(email, password) -> Option<User>
   - update_user_public_key(user_id, public_key)
   - get_user_by_id/email
   ```

2. Implement balance operations:
   ```rust
   - get_or_create_asset(mint_address, decimals, name, symbol)
   - update_balance(user_id, mint_address, amount)
   - get_user_balances(user_id) -> HashMap<mint, (balance, asset)>
   - bulk_update_balances(Vec<(user_id, mint, amount)>)
   ```

3. Implement quote operations:
   ```rust
   - create_quote(user_id, input_mint, output_mint, quote_data, expires_in)
   - get_valid_quote(quote_id, user_id)
   - mark_quote_used(quote_id)
   - cleanup_expired_quotes()
   ```

**Success Criteria:**
- All store tests pass
- Database migrations run successfully
- Connection pooling works properly

#### Step 1.2: Database Schema Validation
**Files to check/update:**
- `migrations/001_initial_schema.sql`

**Tasks:**
1. Verify all required tables exist:
   - users (with public_key field)
   - assets (mint_address, decimals, name, symbol)
   - balances (user_id, asset_id, amount)
   - quotes (quote_data JSONB, expires_at)

2. Add missing indexes for performance:
   ```sql
   CREATE INDEX idx_balances_user_asset ON balances(user_id, asset_id);
   CREATE INDEX idx_quotes_user_expires ON quotes(user_id, expires_at);
   ```

3. Test with sample data insertion

**Success Criteria:**
- All migrations run without errors
- Sample CRUD operations work
- Query performance is acceptable

---

### **Phase 2: MPC Implementation (Days 3-5)**
*Core security layer - distributed key management*

#### Step 2.1: Complete MPC Node Implementation
**Priority: CRITICAL** - Core wallet functionality

**Files to create/modify:**
- `mpc/src/main.rs` - HTTP server with FROST integration
- `mpc/src/tss.rs` - Threshold signing service
- `mpc/src/error.rs` - Comprehensive error handling
- `mpc/src/serialization.rs` - MessagePack serialization
- `mpc/Cargo.toml` - Add FROST dependencies

**Tasks:**
1. Implement FROST Ed25519 key generation:
   ```rust
   - generate_key_share(user_id, threshold, max_participants)
   - store key shares in sled database
   - return aggregated public key
   ```

2. Implement 2-phase distributed signing:
   ```rust
   - sign_phase1(user_id, message) -> (nonces, commitments)
   - sign_phase2(user_id, message, signing_package) -> signature_share
   - aggregate_signature(shares, signing_package) -> final_signature
   ```

3. Create HTTP API endpoints:
   ```
   POST /api/keygen - Generate distributed key
   POST /api/sign-phase1 - First signing round
   POST /api/sign-phase2 - Second signing round  
   POST /api/aggregate - Combine signature shares
   GET /health - Node health check
   ```

4. Implement persistent storage:
   - Key shares stored in sled database per node
   - Session management for signing rounds
   - Recovery mechanisms for failed operations

**Success Criteria:**
- 3 MPC nodes start successfully
- Distributed key generation works
- 2-phase signing produces valid Ed25519 signatures
- Nodes survive restarts with key persistence

#### Step 2.2: MPC Integration Testing
**Files to create:**
- `scripts/test_mpc_integration.sh`
- `scripts/start_mpc_cluster.sh`

**Tasks:**
1. Create test scripts:
   ```bash
   # Start 3 MPC nodes on ports 8001, 8002, 8003
   # Test key generation for sample user
   # Test signing with different message
   # Verify signature with public key
   ```

2. Load testing:
   - Multiple concurrent key generations
   - Multiple concurrent signing operations
   - Node failure scenarios (1 node down, 2 nodes down)

3. Security validation:
   - Verify no single node can sign alone
   - Verify any 2 nodes can complete signing
   - Test with malformed inputs

**Success Criteria:**
- MPC cluster handles 10+ concurrent operations
- Graceful degradation when nodes fail
- All security properties maintained

---

### **Phase 3: Backend API Integration (Days 6-7)**
*Application layer - user-facing services*

#### Step 3.1: Complete MPC Client Service
**Files to create/modify:**
- `backend/src/services/mpc.rs` - MPC coordination client
- `backend/src/services/mod.rs` - Service exports

**Tasks:**
1. Implement MPC client:
   ```rust
   - generate_key(user_id) -> public_key
   - sign_message(user_id, message_hex) -> signature
   - sign_transaction(user_id, tx_hash, tx_data) -> signature
   - health_check() -> node_status
   - check_threshold_availability() -> bool
   ```

2. Add error handling and retries:
   - Network timeouts
   - Node unavailability
   - Partial responses
   - Threshold not met scenarios

3. Implement load balancing:
   - Round-robin node selection
   - Fallback mechanisms
   - Health-based routing

**Success Criteria:**
- Backend can coordinate MPC operations
- Proper error handling for all failure modes
- Load balancing works across nodes

#### Step 3.2: Complete User Routes with MPC
**Files to modify:**
- `backend/src/routes/user.rs` - User management with MPC

**Tasks:**
1. Implement sign-up workflow:
   ```rust
   POST /api/user/signup:
   1. Validate email/password
   2. Create user in database
   3. Trigger MPC key generation across nodes
   4. Update user with aggregated public key
   5. Initialize SOL balance to 0
   6. Return JWT token + user profile
   ```

2. Implement sign-in workflow:
   ```rust
   POST /api/user/signin:
   1. Verify credentials
   2. Generate JWT token
   3. Return user profile with public key
   ```

3. Add user management endpoints:
   ```rust
   GET /api/user/profile - Get current user info
   POST /api/user/regenerate-keys - Re-run MPC keygen if failed
   GET /api/user/wallet-status - Check MPC health & balances
   ```

**Success Criteria:**
- Complete user registration with MPC keys
- JWT authentication works
- Error handling for MPC failures

#### Step 3.3: Authentication Middleware Integration
**Files to verify:**
- `backend/src/middleware/auth.rs` - JWT validation

**Tasks:**
1. Ensure middleware extracts user claims correctly
2. Verify protected routes require valid JWT
3. Test token expiration handling

**Success Criteria:**
- Protected endpoints reject invalid tokens
- User context available in route handlers

---

### **Phase 4: Solana Integration (Days 8-9)**
*Blockchain layer - transaction processing*

#### Step 4.1: Complete Solana Transaction Handling
**Files to create/modify:**
- `backend/src/services/solana.rs` - Solana RPC client
- `backend/src/routes/solana.rs` - Transaction endpoints

**Tasks:**
1. Implement SOL transfer functionality:
   ```rust
   POST /api/solana/send:
   1. Validate recipient address and amount
   2. Create Solana transfer transaction
   3. Extract transaction hash for signing
   4. Coordinate MPC signing of hash
   5. Apply signature to transaction
   6. Broadcast to Solana network
   7. Return transaction signature
   ```

2. Implement SPL token transfers:
   ```rust
   - Support for any SPL token by mint address
   - Proper decimal handling per token
   - Associated token account creation if needed
   ```

3. Add transaction building utilities:
   ```rust
   - build_transfer_transaction(from, to, amount)
   - build_token_transfer_transaction(from, to, mint, amount)
   - extract_signable_hash(transaction)
   - apply_signature_to_transaction(tx, signature)
   ```

**Success Criteria:**
- SOL transfers work end-to-end
- SPL token transfers work
- Proper error handling for insufficient funds
- Transaction signatures are valid

#### Step 4.2: Balance Querying
**Files to create/modify:**
- `backend/src/routes/solana.rs` - Balance endpoints

**Tasks:**
1. Implement balance endpoints:
   ```rust
   GET /api/solana/balance - Get all token balances for user
   GET /api/solana/balance/{mint} - Get specific token balance
   ```

2. For now, use direct RPC queries (will be replaced by indexer):
   ```rust
   - Query SOL balance from user's public key
   - Query token accounts for user's public key
   - Aggregate all token balances
   ```

**Success Criteria:**
- Users can see their SOL balance
- Users can see all their token balances
- Fast response times (<2s)

---

### **Phase 5: Jupiter DEX Integration (Days 10-11)**
*Trading layer - token swaps*

#### Step 5.1: Complete Jupiter Client
**Files to modify:**
- `backend/src/services/jupiter.rs` - Jupiter API client

**Tasks:**
1. Implement quote fetching:
   ```rust
   - get_quote(input_mint, output_mint, amount, slippage)
   - handle route selection and price impact
   - return quote with all metadata
   ```

2. Implement swap transaction building:
   ```rust
   - get_swap_transaction(quote_data, user_pubkey)
   - return serialized transaction for signing
   ```

3. Add quote validation:
   - Check quote expiration
   - Verify price bounds
   - Validate slippage tolerance

**Success Criteria:**
- Can fetch quotes for major token pairs
- Swap transactions build correctly
- Proper error handling for no routes

#### Step 5.2: Complete Swap Workflow
**Files to modify:**
- `backend/src/routes/solana.rs` - Add swap endpoints

**Tasks:**
1. Implement swap quote endpoint:
   ```rust
   POST /api/solana/quote:
   1. Get quote from Jupiter
   2. Store complete quote in database
   3. Return quote ID and preview data
   ```

2. Implement swap execution:
   ```rust
   POST /api/solana/swap:
   1. Retrieve stored quote by ID
   2. Validate quote is still valid (not expired/used)
   3. Build swap transaction from quote
   4. Extract transaction hash for signing
   5. Coordinate MPC signing
   6. Apply signature and broadcast
   7. Mark quote as used
   8. Return transaction signature
   ```

3. Add swap validation:
   - Verify user has sufficient input token balance
   - Check slippage bounds
   - Validate quote hasn't expired

**Success Criteria:**
- Complete quote-to-swap workflow works
- Quotes stored and retrieved correctly
- MPC signing integrated with swaps
- Proper balance validation

---

### **Phase 6: Real-time Indexer (Days 12-14)**
*Data layer - balance tracking*

#### Step 6.1: Yellowstone gRPC Integration
**Files to create/modify:**
- `indexer/src/main.rs` - Main indexer service
- `indexer/src/yellowstone.rs` - gRPC client
- `indexer/src/processor.rs` - Account update processing

**Tasks:**
1. Implement Yellowstone gRPC subscription:
   ```rust
   - Connect to Yellowstone gRPC endpoint
   - Subscribe to account updates for:
     * System Program (SOL transfers)
     * Token Program (SPL transfers)
     * Token-2022 Program (new SPL standard)
   - Handle connection failures and reconnection
   ```

2. Add dynamic account subscription:
   ```rust
   - When new user creates wallet, add their address to subscription
   - When user receives new token, add token account to subscription
   - Efficiently manage subscription lists
   ```

3. Implement update processing:
   ```rust
   - Parse account data for balance changes
   - Extract transaction context (slot, signature)
   - Update database with new balances
   - Handle account creation/closure
   ```

**Success Criteria:**
- Real-time updates for SOL balance changes
- Real-time updates for token balance changes
- Handles network interruptions gracefully

#### Step 6.2: Balance Database Integration
**Files to create/modify:**
- `indexer/src/database.rs` - Database operations for indexer

**Tasks:**
1. Implement balance update logic:
   ```rust
   - update_sol_balance(user_address, new_balance, slot)
   - update_token_balance(user_address, mint, new_balance, slot)
   - discover_new_token(mint_address) -> asset_info
   ```

2. Add batch processing:
   ```rust
   - Batch multiple updates for efficiency
   - Handle duplicate updates (same slot)
   - Maintain update ordering
   ```

3. Asset discovery:
   ```rust
   - When user receives new token, lookup token metadata
   - Store token info (name, symbol, decimals) in assets table
   - Handle tokens without metadata gracefully
   ```

**Success Criteria:**
- Balances update within 1-2 seconds of on-chain changes
- New tokens automatically discovered and tracked
- Database stays consistent with blockchain state

#### Step 6.3: Replace RPC Balance Queries
**Files to modify:**
- `backend/src/routes/solana.rs` - Remove RPC balance queries

**Tasks:**
1. Update balance endpoints to use indexed data:
   ```rust
   - Get balances from database instead of RPC
   - Much faster response times
   - Always up-to-date with indexer
   ```

2. Add balance update webhooks (optional):
   ```rust
   - Notify frontend when balances change
   - WebSocket or Server-Sent Events
   ```

**Success Criteria:**
- Balance queries are fast (<100ms)
- No more direct RPC dependencies for balances
- Real-time balance updates work

---

### **Phase 7: Production Hardening (Days 15-16)**
*Security and reliability layer*

#### Step 7.1: Security Enhancements
**Files to create/modify:**
- `backend/src/middleware/rate_limit.rs` - Rate limiting
- `backend/src/middleware/cors.rs` - CORS configuration
- `mpc/src/auth.rs` - Inter-node authentication

**Tasks:**
1. Add rate limiting:
   ```rust
   - Limit signup requests (prevent spam)
   - Limit transaction requests per user
   - Limit MPC operations per time window
   ```

2. Secure MPC node communication:
   ```rust
   - Add shared secret authentication between nodes
   - Verify request signatures
   - Prevent unauthorized MPC operations
   ```

3. Input validation and sanitization:
   ```rust
   - Validate all email addresses
   - Sanitize all user inputs
   - Prevent SQL injection (already handled by sqlx)
   - Validate Solana addresses
   ```

4. Add audit logging:
   ```rust
   - Log all user actions (signup, signin, transfers, swaps)
   - Log all MPC operations
   - Structured logging for monitoring
   ```

**Success Criteria:**
- API protected against common attacks
- MPC nodes secured against unauthorized access
- Comprehensive audit trail

#### Step 7.2: Error Handling and Recovery
**Files to create/modify:**
- `backend/src/error.rs` - Centralized error handling
- `scripts/health_check.sh` - System health monitoring

**Tasks:**
1. Implement comprehensive error handling:
   ```rust
   - Database connection failures
   - MPC node failures
   - Solana RPC failures
   - Jupiter API failures
   - Network timeouts
   ```

2. Add recovery mechanisms:
   ```rust
   - Retry logic for transient failures
   - Circuit breaker pattern for external services
   - Graceful degradation when services are down
   ```

3. Health monitoring:
   ```rust
   - Database connection health
   - MPC node availability
   - Solana network connectivity
   - Indexer lag monitoring
   ```

4. Add admin endpoints:
   ```rust
   GET /admin/health - System health overview
   GET /admin/stats - Usage statistics
   POST /admin/cleanup - Manual cleanup operations
   ```

**Success Criteria:**
- System handles failures gracefully
- Automatic recovery from transient issues
- Clear health monitoring and alerting

#### Step 7.3: Performance Optimization
**Tasks:**
1. Database optimization:
   ```sql
   - Add missing indexes
   - Optimize slow queries
   - Connection pool tuning
   ```

2. API optimization:
   ```rust
   - Response compression
   - Request caching where appropriate
   - Database query optimization
   ```

3. Load testing:
   ```bash
   - Test with 100+ concurrent users
   - Test MPC performance under load
   - Test indexer with high transaction volume
   ```

**Success Criteria:**
- API responds under 200ms for most operations
- System handles 100+ concurrent users
- Database queries optimized

---

## Testing Strategy

### Unit Tests
- **Store module**: Test all CRUD operations
- **MPC module**: Test key generation and signing
- **Jupiter client**: Test quote and swap logic
- **Authentication**: Test JWT generation/validation

### Integration Tests  
- **End-to-end user signup**: Database + MPC + JWT
- **Complete transaction flow**: Balance check + MPC signing + broadcast
- **Complete swap flow**: Quote + MPC signing + broadcast
- **Indexer integration**: Real-time balance updates

### Load Tests
- **MPC performance**: 50+ concurrent key generations
- **API performance**: 200+ requests/second
- **Database performance**: 1000+ balance updates/second

### Security Tests
- **Authentication bypass attempts**
- **MPC threshold violations**
- **Input validation testing**
- **Rate limiting validation**

---

## Deployment Strategy

### Development Environment
```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: solana_wallet
  
  mpc-node-1:
    build: ./mpc
    ports: ["8001:8001"]
    environment:
      NODE_ID: 1
  
  mpc-node-2:
    build: ./mpc
    ports: ["8002:8002"]
    environment:
      NODE_ID: 2
      
  mpc-node-3:
    build: ./mpc
    ports: ["8003:8003"]
    environment:
      NODE_ID: 3
      
  backend:
    build: ./backend
    ports: ["8080:8080"]
    depends_on: [postgres, mpc-node-1, mpc-node-2, mpc-node-3]
    
  indexer:
    build: ./indexer
    depends_on: [postgres]
```

### Production Considerations
- **Separate MPC nodes on different servers** for true security
- **Database replication and backups**
- **Load balancers for high availability**
- **Monitoring and alerting systems**
- **SSL/TLS termination**

---

## Success Metrics

### Functional Requirements
- ✅ Users can create accounts with MPC-generated keys
- ✅ Users can send SOL and SPL tokens
- ✅ Users can swap tokens via Jupiter
- ✅ Real-time balance updates work
- ✅ 2/3 MPC threshold signing works
- ✅ System handles node failures gracefully

### Performance Requirements  
- ✅ API response times < 200ms
- ✅ Balance updates within 2 seconds of on-chain changes
- ✅ Support 100+ concurrent users
- ✅ MPC operations complete within 5 seconds

### Security Requirements
- ✅ No single point of private key failure
- ✅ MPC nodes cannot sign individually
- ✅ User authentication required for all operations
- ✅ Rate limiting prevents abuse
- ✅ Comprehensive audit logging

---

## Timeline Summary

| Phase | Days | Key Deliverables |
|-------|------|------------------|
| Phase 1 | 1-2 | Complete store operations, database validation |
| Phase 2 | 3-5 | Working MPC cluster with FROST signing |
| Phase 3 | 6-7 | User signup/signin with MPC integration |
| Phase 4 | 8-9 | SOL/SPL token transfers working |
| Phase 5 | 10-11 | Jupiter swaps working end-to-end |
| Phase 6 | 12-14 | Real-time indexer replacing RPC queries |
| Phase 7 | 15-16 | Production hardening and optimization |

**Total Duration: 16 days**

---

## Risk Mitigation

### Technical Risks
1. **MPC complexity** - Use proven FROST implementation, extensive testing
2. **Solana network issues** - Add retry logic, fallback RPC endpoints
3. **Jupiter API limitations** - Handle rate limits, quote expiration
4. **Database performance** - Proper indexing, connection pooling

### Security Risks
1. **Key compromise** - Distributed storage, no single point of failure
2. **Authentication bypass** - JWT validation, input sanitization  
3. **DDoS attacks** - Rate limiting, load balancing
4. **Data corruption** - Database backups, transaction consistency

### Operational Risks
1. **Service downtime** - Health monitoring, automatic failover
2. **Data loss** - Regular backups, replication
3. **Scaling issues** - Load testing, performance optimization
4. **Monitoring gaps** - Comprehensive logging, alerting systems

This implementation plan provides a clear roadmap from your current state to a fully functional MPC Solana wallet service. Each phase builds upon the previous one, with clear success criteria and testing requirements.