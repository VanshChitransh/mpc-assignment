# EXECUTION PLAN — MPC Solana Wallet

Last updated: 2025-10-14

This document lays out a step-by-step plan to complete the project, with implementation instructions, commands, and success criteria.


## Phase 0 — Prerequisites and environment
Objective: Ensure your machine has the required toolchain and services.

Steps:
- Install tools
  - Rust (stable) and cargo
  - sqlx-cli for migrations and offline prepare
  - Docker (for dev Postgres)
  - Optional: solana-cli tools for troubleshooting
- Prepare environment variables
  - .env files exist at root, backend/, indexer/, and mpc/.env.node{1,2,3}
  - Ensure DATABASE_URL points to your dev DB
- Start a local Postgres via Docker
  ```bash
  docker run --name solana-wallet-db \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_USER=postgres \
    -e POSTGRES_DB=solana_wallet \
    -p 5432:5432 -d postgres:15
  ```
- Run migrations
  ```bash
  export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_wallet
  sqlx migrate run
  ```
- Prepare SQLx offline caches
  ```bash
  (cd store && DATABASE_URL=$DATABASE_URL cargo sqlx prepare)
  (cd indexer && DATABASE_URL=$DATABASE_URL cargo sqlx prepare)
  ```

Success criteria:
- `sqlx migrate run` completes successfully
- `cargo sqlx prepare` generated caches for store and indexer


## Phase 1 — Unblock backend builds (feature-gate Solana token deps)
Objective: Resolve the `zeroize` version conflict that blocks backend builds.

Steps:
- Make token crates optional and add a feature flag in `backend/Cargo.toml`:
  ```toml
  [dependencies]
  # ...existing...
  spl-token = { version = "6.0", optional = true }
  spl-associated-token-account = { version = "5.0", optional = true }

  [features]
  default = []
  real_solana = ["spl-token", "spl-associated-token-account"]
  ```
- In `backend/src/blockchain/solana.rs`, guard token-transfer-specific code with `cfg(feature = "real_solana")`:
  ```rust
  #[cfg(feature = "real_solana")]
  use spl_token::instruction as token_instruction;
  #[cfg(feature = "real_solana")]
  use spl_associated_token_account::{
      get_associated_token_address,
      instruction::create_associated_token_account,
  };

  #[cfg(feature = "real_solana")]
  pub async fn create_token_transfer_transaction(...) -> Result<Transaction, SolanaError> {
      // ... implementation ...
  }
  ```
- Default build uses mock_mode=true in `backend/src/main.rs`.
- Verify backend builds:
  ```bash
  cargo build --manifest-path ./backend/Cargo.toml
  ```
- Real mode build when ready:
  ```bash
  cargo build --manifest-path ./backend/Cargo.toml --features real_solana
  ```

Success criteria:
- Default backend build succeeds without `zeroize` conflict
- Real mode compiles with the feature flag when needed


## Phase 2 — Consolidate database access (remove duplication)
Objective: Use the `store` crate consistently from backend; remove `backend/src/store.rs` duplication.

Steps:
- Replace direct SQLx calls in backend routes with `store` crate methods:
  - User signup/signin/profile → `store::user`
  - Quotes → `store::quote`
  - Balances → `store::balance`
- Keep backend route handlers thin: validate input + translate to/from store types
- Delete `backend/src/store.rs` once no references remain
- Build to confirm
  ```bash
  cargo build --manifest-path ./backend/Cargo.toml
  ```

Success criteria:
- Backend compiles using only the `store` crate for DB access
- No references to `backend/src/store.rs`


## Phase 3 — MPC integration alignment (two-phase signing)
Objective: Make backend’s `MpcClient` interoperate with MPC node’s `/api/sign-phase1` and `/api/sign-phase2`.

Steps:
- Update `backend/src/services/mpc.rs` to support real mode:
  - `generate_key(user_id)` → POST `/api/keygen`
  - `sign_message(user_id, message_hex)` two-phase flow:
    1) POST `/api/sign-phase1` with `{ user_id, message }`
    2) POST `/api/sign-phase2` with `{ user_id, session_id, signing_package }`
    3) POST `/api/aggregate` with `{ signature_shares, signing_package }` (or aggregate in backend if you prefer)
- Keep `mock_mode` as default for local dev; gate real mode via env or config
- Run three MPC nodes for manual testing:
  ```bash
  # Terminal 1
  (cd mpc; set -a; source ./.env.node1; set +a; cargo run)
  # Terminal 2
  (cd mpc; set -a; source ./.env.node2; set +a; cargo run)
  # Terminal 3
  (cd mpc; set -a; source ./.env.node3; set +a; cargo run)
  ```

Success criteria:
- Backend can call keygen and two-phase sign across at least two nodes and aggregate a valid signature
- Optional: a backend health check fan-out shows all three nodes up


## Phase 4 — User flows in backend (end-to-end with MPC)
Objective: Solidify signup/signin/profile using `store` + MPC; keep mock defaults, enable real via config.

Steps:
- Signup:
  1) Validate email/password
  2) Create user via `store`
  3) Trigger MPC keygen; store `public_key` to user
  4) Return JWT + user profile
- Signin:
  - Validate credentials via `store::user.authenticate_user`
  - Return JWT + user profile
- Profile:
  - Protected via JWT middleware
  - Returns user profile from `store`
- Test manually:
  ```bash
  curl -X POST http://127.0.0.1:8080/api/user/signup -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","password":"secret123"}'
  ```

Success criteria:
- JWTs issued and validated by middleware
- Public key populated on signup when MPC nodes are available (or a clear error if not)


## Phase 5 — Solana transaction signing and send (real mode)
Objective: Implement real send using MPC signatures on Solana transactions.

Steps:
- Flow for SOL transfer:
  1) Build tx: `SolanaClient::create_transfer_transaction(from, to, amount, memo)`
  2) Extract signable message hash: `transaction.message.serialize()` (derive canonical hash if desired)
  3) Call `MpcClient.sign_message(user_id, message_hash_hex)`
  4) Apply signature: `sign_and_finalize_transaction(tx, signature_hex)`
  5) Broadcast: `broadcast_transaction(SOLANA_RPC_URL, Ok(signed_tx))`
- Validate addresses, amounts, and sufficient balance (RPC) when not in mock mode
- Example request:
  ```bash
  curl -X POST http://127.0.0.1:8080/api/solana/send \
    -H "Authorization: Bearer <JWT>" \
    -H "Content-Type: application/json" \
    -d '{"to":"<dest_pubkey>", "amount":0.001, "memo":null}'
  ```

Success criteria:
- Real RPC broadcast returns a transaction signature on devnet
- Robust error handling for insufficient funds, invalid addresses, and RPC failures


## Phase 6 — Jupiter real integration (quotes and swaps)
Objective: Replace mock Jupiter with real API calls.

Steps:
- Implement real Jupiter client:
  - GET `/v6/quote?inputMint=...&outputMint=...&amount=...&slippageBps=...`
  - POST swap transaction build endpoint (per Jupiter’s latest API)
  - Add timeouts, retries, and validation (price impact, slippage bounds)
- Store each quote with full `quote_data` and `expires_at` via `store::quote::create_quote`
- Swap execution:
  1) Retrieve stored quote by ID; validate not expired/used
  2) Request swap transaction from Jupiter (or build from quote)
  3) Extract signable hash; call MPC signing
  4) Apply signature; broadcast to Solana
  5) Mark quote used
- Example endpoints:
  ```bash
  # Get quote
  curl -X POST http://127.0.0.1:8080/api/solana/quote \
    -H "Authorization: Bearer <JWT>" \
    -H "Content-Type: application/json" \
    -d '{"input_mint":"So111...","output_mint":"EPjF...","amount":"1000000","slippage":0.5}'

  # Execute swap
  curl -X POST http://127.0.0.1:8080/api/solana/swap \
    -H "Authorization: Bearer <JWT>" \
    -H "Content-Type: application/json" \
    -d '{"quote_id":"<uuid>"}'
  ```

Success criteria:
- Quotes returned for common pairs; swaps complete on devnet
- Quotes stored and marked used; validation prevents expired/already-used quotes


## Phase 7 — Indexer (Yellowstone/gRPC) and balance updates
Objective: Implement real-time indexing and switch backend balance endpoints to DB instead of RPC.

Steps:
- Indexer service:
  - Connect to Yellowstone (or mock geyser)
  - Subscribe to SOL balances, SPL Token (Token Program), Token-2022 updates
  - Process updates: update `user_wallets`, `balance_changes`, `token_balances`, `transactions`
  - Persist slot progress in `indexer_state`; implement reconnect/backoff
  - Batch updates and dedupe by slot
- Switch backend balance endpoints to query indexed tables (aggregate by user’s public key(s))

Success criteria:
- Balances reflect on-chain changes within ~1–2s
- Backend balance endpoints no longer use direct RPC


## Phase 8 — Testing and validation
Objective: Add unit, integration, and load tests.

Steps:
- Unit tests:
  - store: CRUD, balances, quotes
  - backend: JWT middleware, input validation, error mapping
  - mpc: serialization/state handling; sign/aggregate pieces where feasible
  - indexer: processors and DB functions
- Integration scripts (add under `scripts/`):
  - `start_mpc_cluster.sh`: run three mpc nodes
  - `test_mpc_integration.sh`: keygen + signing end-to-end
  - `test_backend_e2e.sh`: signup → quote → swap (mock/real configurable)
  - `test_indexer_local.sh`: mock geyser feed to update DB
- Load tests:
  - k6/Gatling or similar to hit `/api/user/*` and `/api/solana/*`

Commands:
```bash
TEST_DATABASE_URL=$DATABASE_URL cargo test --manifest-path ./store/Cargo.toml --no-fail-fast
cargo test --manifest-path ./indexer/Cargo.toml --no-fail-fast
cargo test --manifest-path ./mpc/Cargo.toml --no-fail-fast
# backend tests after Phase 2/3 refactors
```

Success criteria:
- Tests pass; load targets achieved (e.g., 200 RPS API, MPC 50+ concurrent keygens in mock)


## Phase 9 — Security and production readiness
Objective: Add rate limiting, inter-node auth, input validation, and observability.

Steps:
- Backend middleware:
  - Rate limiting per IP/user on sensitive routes
  - CORS configuration restricted to your frontend origins
  - Structured logging with tracing and correlation IDs
- MPC nodes:
  - Inter-node auth via shared secret or mTLS
  - Reject unauthenticated key/sign requests
- Input validation:
  - Validate emails, Solana addresses, and amounts
  - Sanitize and validate JSON inputs everywhere
- Audit logging:
  - Log signup/signin, keygen, sign, transfer, swap
- Observability:
  - `/health` endpoints for backend, MPC, indexer
  - Metrics: success/error counts, latencies, indexer lag

Success criteria:
- Protected against common abuse (rate limits, invalid inputs)
- MPC endpoints secure against unauthorized access
- Useful logs and metrics for monitoring


## Phase 10 — Docker Compose and one-command dev
Objective: Spin full stack locally with a single command.

Steps:
- Create `docker-compose.yml` with services: postgres, mpc-node-1/2/3, backend, indexer
- Containerize each crate; pass envs from sample `.env` files; ensure DB readiness and migrations run

Example skeleton:
```yaml
version: "3.8"
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_USER: postgres
      POSTGRES_DB: solana_wallet
    ports: ["5432:5432"]

  mpc-node-1:
    build: ./mpc
    environment:
      NODE_ID: 1
      BIND_ADDRESS: 0.0.0.0:8001
      PEER_NODES: http://mpc-node-1:8001,http://mpc-node-2:8002,http://mpc-node-3:8003
    ports: ["8001:8001"]
  # repeat for node-2 and node-3 on 8002/8003

  backend:
    build: ./backend
    environment:
      DATABASE_URL: postgresql://postgres:postgres@postgres:5432/solana_wallet
      MPC_NODES: http://mpc-node-1:8001,http://mpc-node-2:8002,http://mpc-node-3:8003
      SOLANA_RPC_URL: https://api.devnet.solana.com
      JUPITER_API_URL: https://quote-api.jup.ag/v6
    ports: ["8080:8080"]
    depends_on: ["postgres", "mpc-node-1", "mpc-node-2", "mpc-node-3"]

  indexer:
    build: ./indexer
    environment:
      DATABASE_URL: postgresql://postgres:postgres@postgres:5432/solana_wallet
```

Success criteria:
- `docker compose up` brings the entire stack online
- Backend endpoints respond; MPC nodes healthy; DB populated


## Phase 11 — CI/CD and quality gates
Objective: Automate checks and promotions.

Steps:
- GitHub Actions (or similar):
  - rustfmt + clippy gates
  - cargo build and cargo test for each crate
  - Spin ephemeral Postgres for integration tests; run migrations; run store/indexer tests
  - Build Docker images on main/tag
- SQLx prepare in CI to ensure offline caches up-to-date:
  ```bash
  (cd store && DATABASE_URL=${{ secrets.CI_DATABASE_URL }} cargo sqlx prepare --check)
  (cd indexer && DATABASE_URL=${{ secrets.CI_DATABASE_URL }} cargo sqlx prepare --check)
  ```

Success criteria:
- PRs blocked unless code formats, lints, builds, and tests pass
- Images published on tags


## Phase 12 — Cutover to real mode
Objective: Disable mocks, enable real integrations.

Steps:
- Toggle `mock_mode=false` in `backend/src/main.rs` for:
  - `MpcClient`
  - `SolanaClient`
  - `JupiterClient`
- Build with `real_solana` feature:
  ```bash
  cargo build --manifest-path ./backend/Cargo.toml --features real_solana
  ```
- Validate end-to-end on devnet:
  - SOL send works
  - Jupiter quotes/swaps work
  - MPC signatures verify on-chain

Success criteria:
- End-to-end transaction and swap flows work against devnet
- Indexer keeps balances up to date
- Logs/metrics show health


## Validation checklist per phase
- Phase 0–1: Builds succeed; DB and SQLx prepared
- Phase 2: No backend-local store code remains; routes use `store` crate
- Phase 3: MPC two-phase signing integrated (mock and real)
- Phase 4: Auth flows and JWT middleware verified via tests and manual curl
- Phase 5: Real SOL send works; signature paths verified
- Phase 6: Real Jupiter quotes/swaps; quotes stored/validated; swaps marked used
- Phase 7: Indexer updates DB; backend reads from DB; RPC independent for balances
- Phase 8: Tests pass; load targets met
- Phase 9: Security controls in place; logs/metrics available
- Phase 10: Docker Compose one-command dev
- Phase 11: CI/CD green on PRs; images published
- Phase 12: Mocks off; real integrations on


## Notes on secrets handling
- Never hardcode secrets in code or commands; use environment variables
- In scripts/CI, reference placeholders like `{{SECRET_NAME}}` and load via your secret manager
