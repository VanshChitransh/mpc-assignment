# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

Project overview
- Tech stack: Rust (Actix-web, sqlx, tokio), Solana SDK 2.0, SPL Token, JWT (jsonwebtoken), sled (embedded DB), FROST Ed25519, reqwest, tracing. The repo contains four Rust crates: backend (HTTP API), store (database ops), mpc (threshold signing node), indexer (Yellowstone/GRPC-based indexer).
- No cargo workspace at the root. Build/run each crate from its directory.
- Environment: .env files exist at repo root, backend/, indexer/, and sample envs for MPC nodes in mpc/.env.node{1,2,3}.

Common commands
- Build (per crate)
  - backend: cd backend && cargo build
  - store: cd store && cargo build
  - mpc: cd mpc && cargo build
  - indexer: cd indexer && cargo build
- Run
  - Backend API (mock integrations enabled in code):
    - cd backend && cargo run
    - Serves on http://127.0.0.1:8080 (see backend/.env for overrides)
  - MPC nodes (run in separate terminals; use inline envs or copy the node env into ./.env before running):
    - Node 1: (cd mpc && NODE_ID=1 BIND_ADDRESS=127.0.0.1:8001 DATA_DIR=./data/node1 PEER_NODES="http://localhost:8001,http://localhost:8002,http://localhost:8003" cargo run)
    - Node 2: (cd mpc && NODE_ID=2 BIND_ADDRESS=127.0.0.1:8002 DATA_DIR=./data/node2 PEER_NODES="http://localhost:8001,http://localhost:8002,http://localhost:8003" cargo run)
    - Node 3: (cd mpc && NODE_ID=3 BIND_ADDRESS=127.0.0.1:8003 DATA_DIR=./data/node3 PEER_NODES="http://localhost:8001,http://localhost:8002,http://localhost:8003" cargo run)
    - Alternatively, in zsh you can source the sample: (cd mpc; set -a; source .env.node1; set +a; cargo run)
  - Indexer (requires a Postgres DB and a migrations folder at indexer/migrations referenced by sqlx::migrate!("./migrations")):
    - cd indexer && cargo run
    - The example env is in indexer/.env (DATABASE_URL, YELLOWSTONE_ENDPOINT, etc.).
- Test
  - Run all tests in a crate:
    - cd backend && cargo test
    - cd store && cargo test
    - cd mpc && cargo test
    - cd indexer && cargo test
  - Run a single integration test file (backend examples):
    - cd backend && cargo test --test auth_middleware
  - Run a single test by name (substring match):
    - cd backend && cargo test test_public_endpoint_no_auth_required
- Lint and format (run in each crate)
  - cargo fmt --all
  - cargo clippy --all-targets -- -D warnings
- SQLx offline compilation (when DB is unavailable)
  - Some crates hint at SQLX_OFFLINE support. If needed: export SQLX_OFFLINE=true before building store or indexer.

High-level architecture
- Crates and roles
  - backend: Actix-web HTTP API exposing authentication, wallet, and Solana endpoints; composes services (MPC, Jupiter) and a Solana blockchain client. Key modules:
    - src/main.rs: wires AppState with Store, MpcClient, JupiterClient, SolanaClient; registers routes and JWT middleware; loads env via dotenv.
    - src/middleware/auth.rs: JwtAuth (token gen/validation) + AuthMiddleware; injects Claims and user_id into request extensions; public routes: /health, /api/user/signup, /api/user/signin.
    - src/routes/
      - user.rs: signup/signin/profile; triggers MPC keygen on signup and persists public_key.
      - solana.rs: balance/quote/swap/send endpoints; balance aggregates SOL and a few common SPL tokens via SolanaClient; quote persists quotes in DB; swap marks quote used (mock TX id).
      - health.rs: health check including DB status via Store.
      - solana_v1.rs: v1-style endpoints (derive address, transfer) with Prometheus metrics. Note: not currently wired in main.rs.
    - src/services/
      - mpc.rs: client for MPC operations; supports real and mock modes (mock_mode currently enabled in main.rs). In mock mode it generates random bs58 keys/signatures.
      - jupiter.rs: Jupiter quotes/swaps; mock mode synthesizes quotes and base64 transactions.
    - src/blockchain/solana.rs: SolanaClient wrapper (mock or real). Real mode uses RpcClient to fetch balances, build SOL/SPL transfer transactions, and broadcast. Exposes helpers to extract message hash and finalize transactions.
    - src/store.rs: Minimal DB access used by backend (users, quotes, assets fetch/update) wrapping a PgPool.
    - Tests under backend/tests cover auth middleware and Solana blockchain helpers (note: one test file references symbols that have since been renamed; see “Gotchas”).
  - store: Database access layer (sqlx) and shared models.
    - src/models.rs: User, Asset, Balance, Quote, etc., with helper structs (e.g., BalanceWithAsset) and error enums.
    - src/user.rs: User-related operations (create/authenticate/update/list) on a Store facade; uses bcrypt for password hashing.
    - src/balance.rs: Balance queries and updates; ensures Asset rows exist and provides helpers to update/get balances by mint.
  - mpc: Standalone Actix-web service implementing a FROST Ed25519-based threshold signing node.
    - src/tss.rs: ThresholdSigningService that stores key shares and signing state in sled; provides key generation, round1/round2 signing, and aggregation stubs.
    - src/main.rs: HTTP API for keygen, aggregate keys, sign rounds, aggregate signature; loads env via dotenv; designed to run multiple nodes with different NODE_IDs and DATA_DIRs.
  - indexer: GRPC client to (mock) Yellowstone to subscribe to accounts/transactions and write balance/tx data to Postgres.
    - src/yellowstone.rs: YellowstoneClient abstraction (using a mock geyser client here), subscription management, and update streaming; produces YellowstoneUpdate enums.
    - src/processor.rs: Parses updates, detects balance changes, writes balance_changes, user_wallets, token_balances, and transactions rows via sqlx.
    - src/main.rs: Initialization (config via env, migrations via sqlx::migrate!("./migrations")), health/metrics/cleanup loops, and the main processing loop.

Environment and configuration
- Root .env: default DATABASE_URL, JWT, MPC_NODES, RUST_LOG, SOLANA_RPC_URL, JUPITER_API_URL, rate limits.
- backend/.env: per-API service config (DATABASE_URL, JWT_SECRET, MPC_NODES, SOLANA/JUPITER URLs, RUST_LOG).
- indexer/.env: database URL plus Yellowstone endpoint/token and logging flags.
- mpc/.env.node{1,2,3}: sample per-node configs (NODE_ID, BIND_ADDRESS, DATA_DIR, PEER_NODES).

Implementation notes and gotchas
- No Cargo workspace is configured. Use per-crate cargo commands.
- Top-level Cargo.toml (simple-server) doesn’t correspond to any source file in the repo and can be ignored; operate within backend/, store/, mpc/, indexer/.
- Backend currently initializes Solana/MPC/Jupiter clients in mock mode (hard-coded ‘true’ in backend/src/main.rs). Real integrations require code changes and appropriate env.
- Indexer expects indexer/migrations for sqlx::migrate! macro. If missing, either add migrations or build with SQLX_OFFLINE=true for development-only compilation.
- Tests: backend/tests/solana_blockchain_tests.rs references types that don’t exist anymore (SolanaBlockchain vs SolanaClient). Prefer running the auth middleware tests; update or skip outdated tests as needed.
- The docs in docs/README.md mention scripts (e.g., start_mpc_cluster.sh) and directories (migrations/) that are not present. Follow the commands above instead.

Reference: important docs in this repo
- docs/README.md: Overall system design, phases, and API descriptions.
- tests/README.md: Test organization and intended flows (some scripts referenced are not included).
