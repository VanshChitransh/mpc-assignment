# Solana Wallet Backend - Complete Setup & Architecture Index

## Project Overview

This is a **Solana wallet backend system** with 4 main components providing a complete wallet infrastructure with MPC (Multi-Party Computation) threshold signing, real-time blockchain indexing, and DEX integration.

## Architecture Components

### 1. **Store Crate** (`store/`) - Core Data Layer
- **Technology**: PostgreSQL + SQLx
- **Purpose**: Central data management for users, balances, assets, and quotes
- **Database**: PostgreSQL with comprehensive schema and performance indexes

### 2. **Backend Service** (`backend/`) - HTTP API Server  
- **Technology**: Actix-web + JWT authentication
- **Purpose**: REST API for wallet operations, user management, and trading
- **Port**: 8080 (configurable)

### 3. **MPC Service** (`mpc/`) - Multi-Party Computation
- **Technology**: Rust + Ed25519 threshold signing
- **Purpose**: Distributed key generation and transaction signing
- **Ports**: 8001, 8002, 8003 (3-node cluster)

### 4. **Indexer Service** (`indexer/`) - Real-time Blockchain Monitor
- **Technology**: Yellowstone GRPC + PostgreSQL
- **Purpose**: Real-time Solana blockchain monitoring and balance tracking
- **Database**: Separate PostgreSQL instance for indexing data

## Database Schema

### Main Schema (`migrations/001_initial_schema.sql`)

#### Core Tables:
- **`users`** - User accounts with MPC public keys
  - `id`, `email`, `password_hash`, `public_key`, `created_at`, `updated_at`
- **`assets`** - Supported tokens (SOL, USDC, USDT, etc.)
  - `id`, `mint_address`, `decimals`, `name`, `symbol`, `logo_url`
- **`balances`** - User token balances (stored in smallest units)
  - `id`, `user_id`, `asset_id`, `amount`, `created_at`, `updated_at`
- **`quotes`** - Jupiter swap quotes with expiration
  - `id`, `user_id`, `input_mint`, `output_mint`, `in_amount`, `out_amount`, `quote_data`, `expires_at`, `used`
- **`keyshares`** - MPC key shares (encrypted)
  - `user_id`, `public_key`, `private_key`, `created_at`

#### Performance Indexes (`migrations/002_performance_indexes.sql`):
- Composite indexes for user-asset lookups
- Partial indexes for active quotes
- Case-insensitive email lookups
- Recent data queries optimization

### Indexer Schema (`indexer/migrations/001_initial.sql`)

#### Indexing Tables:
- **`user_wallets`** - Wallet addresses and SOL balances
- **`balance_changes`** - Historical balance tracking
- **`token_balances`** - SPL token balances
- **`transactions`** - Transaction details
- **`indexer_state`** - Indexer state management
- **`subscription_metrics`** - Performance metrics

## Store Crate Deep Dive

### Core Structure (`store/src/lib.rs`)

```rust
pub struct Store {
    pub pool: PgPool,
}

impl Store {
    // Database management
    pub async fn new_pool(database_url: &str) -> Result<PgPool, sqlx::Error>
    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError>
    pub async fn from_env() -> Result<Self, Box<dyn std::error::Error>>
    
    // Health and monitoring
    pub async fn health_check(&self) -> Result<(), sqlx::Error>
    pub async fn detailed_health_check(&self) -> Result<HealthStatus, sqlx::Error>
    pub async fn get_store_stats(&self) -> Result<StoreStats, Box<dyn std::error::Error>>
    
    // Asset management
    pub async fn initialize_default_assets(&self) -> Result<(), Box<dyn std::error::Error>>
    pub async fn get_all_balances(&self, user_id: &Uuid) -> Result<Vec<BalanceWithAsset>, BalanceError>
    
    // Maintenance
    pub async fn maintenance_cleanup(&self) -> Result<MaintenanceResult, Box<dyn std::error::Error>>
}
```

### User Management (`store/src/user.rs`)

**Exposed Functions:**
- `create_user(request: CreateUserRequest) -> Result<User, UserError>`
- `authenticate_user(email: &str, password: &str) -> Result<User, UserError>`
- `get_user_by_id(user_id: &Uuid) -> Result<User, UserError>`
- `get_user_by_email(email: &str) -> Result<User, UserError>`
- `update_user_public_key(user_id: &Uuid, public_key: &str) -> Result<(), UserError>`
- `get_user_public_key(user_id: &Uuid) -> Result<Option<String>, UserError>`
- `update_user_password(user_id: &Uuid, new_password: &str) -> Result<(), UserError>`
- `delete_user(user_id: &Uuid) -> Result<(), UserError>`
- `get_user_stats() -> Result<UserStats, UserError>`
- `list_users(offset: i64, limit: i64) -> Result<Vec<User>, UserError>`
- `get_users_without_keys() -> Result<Vec<User>, UserError>`

**Helper Functions:**
- Password hashing with bcrypt
- Email validation and normalization
- User profile conversion

### Balance Operations (`store/src/balance.rs`)

**Exposed Functions:**
- `get_sol_balance(user_id: &Uuid) -> Result<i64, BalanceError>`
- `get_token_balances(user_id: &Uuid) -> Result<Vec<BalanceWithAsset>, BalanceError>`
- `get_balance_for_asset(user_id: &Uuid, asset_id: &Uuid) -> Result<i64, BalanceError>`
- `update_balance(user_id: &Uuid, asset_id: &Uuid, new_amount: i64) -> Result<(), BalanceError>`
- `adjust_balance(user_id: &Uuid, asset_id: &Uuid, amount_delta: i64) -> Result<i64, BalanceError>`
- `initialize_user_balances(user_id: &Uuid) -> Result<(), BalanceError>`
- `get_or_create_asset(mint_address: &str, decimals: i32, name: Option<String>, symbol: Option<String>) -> Result<Asset, BalanceError>`
- `get_asset_by_mint(mint_address: &str) -> Result<Asset, BalanceError>`
- `get_asset_by_symbol(symbol: &str) -> Result<Asset, BalanceError>`
- `get_all_assets() -> Result<Vec<Asset>, BalanceError>`
- `add_asset(mint_address: String, decimals: i32, name: String, symbol: String, logo_url: Option<String>) -> Result<Asset, BalanceError>`
- `check_sufficient_balance(user_id: &Uuid, asset_id: &Uuid, required_amount: i64) -> Result<bool, BalanceError>`
- `bulk_update_balances(updates: Vec<(Uuid, Uuid, i64)>) -> Result<(), BalanceError>`
- `update_balance_by_mint(user_id: &Uuid, mint_address: &str, new_amount: i64) -> Result<(), BalanceError>`
- `get_balance_by_mint(user_id: &Uuid, mint_address: &str) -> Result<i64, BalanceError>`

**Helper Functions:**
- Balance validation and error handling
- Asset existence checks
- Transaction management for bulk operations

### Quote Management (`store/src/quote.rs`)

**Exposed Functions:**
- `store_quote(user_id: &Uuid, input_mint: &str, output_mint: &str, in_amount: i64, out_amount: i64, quote_data: Value, expires_in_seconds: i64) -> Result<Quote, QuoteError>`
- `create_quote(user_id: Uuid, input_mint: String, output_mint: String, quote_data: Value, expires_in: i64) -> Result<Quote, QuoteError>`
- `get_quote(quote_id: &Uuid) -> Result<Quote, QuoteError>`
- `get_valid_quote(quote_id: &Uuid, user_id: &Uuid) -> Result<Quote, QuoteError>`
- `mark_quote_used(quote_id: &Uuid) -> Result<(), QuoteError>`
- `get_user_quotes(user_id: &Uuid, limit: Option<i64>) -> Result<Vec<Quote>, QuoteError>`
- `cleanup_expired_quotes() -> Result<u64, QuoteError>`
- `cleanup_old_used_quotes(days_old: i32) -> Result<u64, QuoteError>`
- `get_quote_stats() -> Result<QuoteStats, QuoteError>`

**Helper Functions:**
- Quote expiration checking
- Jupiter quote data parsing
- Quote validation logic

### Data Models (`store/src/models.rs`)

**Core Structs:**
```rust
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub public_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Asset {
    pub id: Uuid,
    pub mint_address: String,
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Balance {
    pub id: Uuid,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub asset_id: Uuid,
}

pub struct Quote {
    pub id: Uuid,
    pub user_id: Uuid,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: i64,
    pub out_amount: i64,
    pub quote_data: serde_json::Value,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used: bool,
    pub updated_at: DateTime<Utc>,
}
```

**Error Types:**
- `UserError` - User-related errors
- `BalanceError` - Balance operation errors  
- `QuoteError` - Quote management errors

**Constants:**
- `LAMPORTS_PER_SOL: i64 = 1_000_000_000`
- `SOL_MINT`, `USDC_MINT`, `USDT_MINT` - Token mint addresses
- `DEFAULT_QUOTE_EXPIRATION_SECONDS: i64 = 300`

## Backend API Structure

### Main Server (`backend/src/main.rs`)

**Configuration:**
- Database connection pooling
- JWT authentication setup
- Service initialization (MPC, Jupiter, Solana clients)
- CORS configuration
- Request logging

**AppState:**
```rust
pub struct AppState {
    pub db: PgPool,
    pub jwt_auth: JwtAuth,
    pub mpc_client: MpcClient,
    pub jupiter_client: JupiterClient,
    pub solana_client: SolanaClient,
}
```

### API Routes

#### User Routes (`/api/user/`)
- **POST** `/signup` - User registration with MPC key generation
- **POST** `/signin` - User authentication with JWT token
- **GET** `/profile` - Get authenticated user profile

#### Solana Routes (`/api/solana/`)
- **GET** `/balance` - Get user's SOL and token balances
- **POST** `/quote` - Get Jupiter swap quote
- **POST** `/swap` - Execute swap transaction
- **POST** `/send` - Send tokens to another address

### Services

#### Jupiter Integration (`backend/src/services/jupiter.rs`)
```rust
pub struct JupiterClient {
    client: Client,
    base_url: String,
    default_slippage_bps: u16,
}

impl JupiterClient {
    pub async fn get_quote(&self, input_mint: &str, output_mint: &str, amount: u64, slippage_bps: Option<u16>) -> Result<JupiterQuoteResponse, JupiterError>
    pub async fn get_swap_transaction(&self, quote_response: Value, user_public_key: &str) -> Result<JupiterSwapResponse, JupiterError>
}
```

#### MPC Client (`backend/src/services/mpc.rs`)
```rust
pub struct MpcClient {
    client: Client,
    nodes: Vec<String>,
    threshold: u32,
    request_timeout: Duration,
}

impl MpcClient {
    pub async fn generate_key(&self, user_id: &Uuid) -> Result<String, MpcError>
    pub async fn sign_transaction(&self, user_id: &Uuid, message_hash: &str, transaction_data: &str) -> Result<String, MpcError>
    pub async fn get_cluster_status(&self) -> ClusterStatus
}
```

#### Solana Client (`backend/src/services/solana.rs`)
```rust
pub struct SolanaClient {
    client: Client,
    rpc_url: String,
}

impl SolanaClient {
    pub async fn build_sol_transfer(&self, from_pubkey: &str, to_pubkey: &str, lamports: u64) -> Result<UnsignedTransaction, SolanaError>
    pub async fn build_token_transfer(&self, from_pubkey: &str, to_pubkey: &str, mint: &str, amount: u64) -> Result<UnsignedTransaction, SolanaError>
    pub async fn broadcast_transaction(&self, transaction_data: &str, signatures: Vec<String>) -> Result<String, SolanaError>
}
```

### Authentication Middleware (`backend/src/middleware/auth.rs`)

**JWT Implementation:**
- HS256 algorithm
- 24-hour token expiration
- Claims: `sub` (user_id), `username`, `exp`, `iat`
- Automatic token validation for protected routes
- Public routes: `/api/user/signup`, `/api/user/signin`, `/health`

## MPC Service Architecture

### Main Service (`mpc/src/main.rs`)

**Configuration:**
- Node ID and bind address
- Data directory for key storage
- Peer node URLs
- Threshold and total parties

**API Endpoints:**
- **GET** `/health` - Node health check
- **POST** `/generate` - Key generation request
- **POST** `/aggregate-keys` - Aggregate public keys
- **POST** `/agg-send-step1` - Signing step 1
- **POST** `/agg-send-step2` - Signing step 2

### Threshold Signing Service (`mpc/src/tss.rs`)

```rust
pub struct ThresholdSigningService {
    pub node_id: u32,
    pub db: Arc<Db>,
    pub peer_nodes: Vec<String>,
    pub signing_sessions: Arc<RwLock<BTreeMap<String, SigningState>>>,
    pub client: reqwest::Client,
}

impl ThresholdSigningService {
    pub async fn generate_key_share(&self, user_id: &Uuid, threshold: u16, total_parties: u16) -> Result<(), MpcError>
    pub async fn get_public_key(&self, user_id: &Uuid) -> Result<Option<String>, MpcError>
    pub async fn sign_step1(&self, user_id: &Uuid, message: &[u8]) -> Result<(), MpcError>
    pub async fn sign_step2(&self, user_id: &Uuid, message: &[u8]) -> Result<String, MpcError>
}
```

**Key Features:**
- Ed25519 key generation and signing
- Sled database for persistent key storage
- Two-phase signing protocol
- Session management for signing operations
- Inter-node communication for threshold operations

### Data Structures (`mpc/src/serialization.rs`)

```rust
pub struct KeyShare {
    pub user_id: Uuid,
    pub node_id: u32,
    pub participant_id: [u8; 2],
    pub key_package: Vec<u8>,
    pub public_key: String,
    pub threshold: u16,
    pub total_parties: u16,
    pub created_at: DateTime<Utc>,
}

pub struct SigningState {
    pub session_id: String,
    pub user_id: Uuid,
    pub message: Vec<u8>,
    pub nonces: Vec<u8>,
    pub commitments: Vec<u8>,
    pub signature_share: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}
```

## Indexer Service Architecture

### Main Service (`indexer/src/main.rs`)

**Configuration:**
- Database URL for indexer data
- Yellowstone GRPC endpoint
- Commitment level (Processed/Confirmed/Finalized)
- Reconnection settings
- Batch processing configuration

**Main Loops:**
- **Processing Loop** - Handle blockchain updates
- **Health Check Loop** - Monitor service health
- **Metrics Loop** - Report performance metrics
- **Cleanup Loop** - Remove old data

### Yellowstone Client (`indexer/src/yellowstone.rs`)

```rust
pub struct YellowstoneClient {
    endpoint: String,
    token: Option<String>,
    client: Option<GeyserGrpcClient<MockInterceptor>>,
    monitored_addresses: Vec<String>,
    commitment: CommitmentLevel,
}

impl YellowstoneClient {
    pub async fn subscribe_to_addresses(&mut self, addresses: Vec<String>) -> Result<(), YellowstoneError>
    pub async fn next_update(&mut self) -> Result<Option<YellowstoneUpdate>, YellowstoneError>
    pub async fn health_check(&mut self) -> Result<bool, YellowstoneError>
    pub async fn reconnect(&mut self) -> Result<(), YellowstoneError>
}
```

**Features:**
- GRPC subscription to Solana blockchain
- Account and transaction monitoring
- Automatic reconnection handling
- Mock implementation for development

### Transaction Processor (`indexer/src/processor.rs`)

```rust
pub struct TransactionProcessor {
    db_pool: PgPool,
}

impl TransactionProcessor {
    pub async fn process_update(&self, update: YellowstoneUpdate) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    async fn process_account_update(&self, update: AccountUpdate) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    async fn process_transaction_update(&self, update: TransactionUpdate) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    async fn process_spl_token_account(&self, update: &AccountUpdate) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
}
```

**Processing Logic:**
- Balance change detection
- Transaction type classification
- SPL token account parsing
- Historical data storage

### Database Manager (`indexer/src/database.rs`)

```rust
pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    pub async fn get_monitoring_addresses(&self) -> Result<Vec<String>, sqlx::Error>
    pub async fn upsert_user_wallet(&self, user_id: &uuid::Uuid, address: &str, balance: i64) -> Result<(), sqlx::Error>
    pub async fn get_stats(&self) -> Result<DatabaseStats, sqlx::Error>
    pub async fn cleanup_old_data(&self, days: i32) -> Result<u64, sqlx::Error>
}
```

## Dependency Map

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Backend   │───▶│    Store    │───▶│ PostgreSQL  │
│  (Actix)    │    │   (SQLx)    │    │   (Main)    │
└─────────────┘    └─────────────┘    └─────────────┘
       │                   │
       ▼                   ▼
┌─────────────┐    ┌─────────────┐
│     MPC     │    │  Indexer    │
│  (Threshold │    │(Yellowstone)│
│   Signing)  │    │             │
└─────────────┘    └─────────────┘
       │                   │
       ▼                   ▼
┌─────────────┐    ┌─────────────┐
│   Solana    │    │ PostgreSQL  │
│    RPC      │    │ (Indexer)   │
└─────────────┘    └─────────────┘
       │
       ▼
┌─────────────┐
│   Jupiter   │
│     DEX     │
└─────────────┘
```

## Testing Infrastructure

### Store Testing (`store/src/bin/`)

#### `test.rs` - Comprehensive Store Testing
- Health check validation
- User creation and authentication
- Balance operations
- Quote management
- Asset operations
- Statistics and maintenance

#### `verify_db.rs` - Database Verification
- Basic connectivity tests
- Table existence validation
- Default asset verification
- User operations testing
- Quote storage testing

#### `schema_validation.rs` - Schema Validation & Performance
- Schema structure validation
- Performance index application
- Sample data insertion
- Query performance testing
- Execution plan analysis

### Unit Tests
- Each module contains comprehensive unit tests
- Integration tests for API endpoints
- Error handling validation
- Edge case coverage

## Environment Configuration

### Required Environment Variables

#### Database
- `DATABASE_URL` - Main PostgreSQL connection string
- `TEST_DATABASE_URL` - Test database connection string

#### Backend Service
- `JWT_SECRET` - Secret key for JWT token signing
- `BIND_ADDRESS` - Server bind address (default: 127.0.0.1:8080)

#### MPC Service
- `NODE_ID` - Unique node identifier (1, 2, 3)
- `BIND_ADDRESS` - Node bind address (default: 127.0.0.1:800{NODE_ID})
- `DATA_DIR` - Data directory for key storage
- `PEER_NODES` - Comma-separated peer node URLs
- `MPC_THRESHOLD` - Threshold for signing (default: 2)

#### Indexer Service
- `DATABASE_URL` - Indexer PostgreSQL connection string
- `YELLOWSTONE_ENDPOINT` - Yellowstone GRPC endpoint
- `YELLOWSTONE_TOKEN` - Authentication token (optional)
- `COMMITMENT_LEVEL` - Blockchain commitment level
- `HEALTH_CHECK_INTERVAL_SECONDS` - Health check interval
- `BATCH_SIZE` - Processing batch size

#### External Services
- `JUPITER_API_URL` - Jupiter DEX API URL
- `SOLANA_RPC_URL` - Solana RPC endpoint

## Setup Instructions

### 1. Database Setup
```bash
# Create PostgreSQL databases
createdb solana_wallet
createdb solana_wallet_indexer

# Run migrations
cd store && cargo run --bin verify_db
cd indexer && cargo run --bin indexer
```

### 2. MPC Cluster Setup
```bash
# Start MPC nodes (3 terminals)
NODE_ID=1 cargo run --bin mpc
NODE_ID=2 cargo run --bin mpc  
NODE_ID=3 cargo run --bin mpc
```

### 3. Backend Service
```bash
# Start API server
cargo run --bin backend
```

### 4. Indexer Service
```bash
# Start blockchain indexer
cargo run --bin indexer
```

## Key Integration Points

1. **Backend ↔ Store**: Direct dependency for all data operations
2. **Backend ↔ MPC**: HTTP client for threshold signing operations
3. **Backend ↔ Jupiter**: HTTP client for DEX quotes and swaps
4. **Backend ↔ Solana**: RPC client for transaction broadcasting
5. **Indexer ↔ Store**: Separate database for real-time indexing
6. **MPC ↔ MPC**: Inter-node communication for threshold operations

## Performance Considerations

### Database Optimization
- Connection pooling with configurable limits
- Comprehensive indexing strategy
- Query optimization with execution plan analysis
- Regular maintenance and cleanup operations

### Caching Strategy
- In-memory session management for MPC
- Connection pooling for external services
- Efficient data structures for real-time processing

### Monitoring
- Health check endpoints for all services
- Performance metrics collection
- Error tracking and logging
- Cluster status monitoring for MPC

## Security Features

### Authentication & Authorization
- JWT-based authentication
- Password hashing with bcrypt
- Token expiration and validation
- Protected route middleware

### MPC Security
- Threshold signing for transaction security
- Distributed key generation
- Encrypted key storage
- Session-based signing operations

### Data Protection
- Input validation and sanitization
- SQL injection prevention with SQLx
- Error handling without information leakage
- Secure configuration management

This comprehensive setup provides a complete Solana wallet infrastructure with enterprise-grade security, real-time monitoring, and scalable architecture.
