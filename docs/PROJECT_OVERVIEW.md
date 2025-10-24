# MPC Solana Wallet — Project Overview and Status

Last updated: 2025-10-14

## What we are building
A multi-party computation (MPC) wallet service for Solana (a mini-Fireblocks). Users never control raw private keys. Instead, keys are distributed across MPC nodes that collaboratively generate and sign, providing strong security properties (no single point of key compromise, threshold signing, survivability under node failure).

## High-level architecture (current repo)
- backend/ — Actix-web HTTP API for auth, wallet, Solana, and Jupiter endpoints. Composes:
  - JwtAuth middleware for authentication
  - MpcClient (mock/real modes)
  - JupiterClient (mock mode today)
  - SolanaClient (mock on by default; real logic implemented)
- store/ — Database access layer (sqlx) and models:
  - Users: create/auth/read/update
  - Balances: per-asset updates, get-or-create asset by mint
  - Quotes: create/get/validate/mark-used/cleanup
- mpc/ — Actix-web MPC node with FROST Ed25519-based keygen/signing (sled persistence):
  - /health, /api/keygen, /api/sign-phase1, /api/sign-phase2, /api/aggregate
- indexer/ — Yellowstone/“geyser”-style client skeleton for subscribing to updates and writing DB state. Basic unit tests present.
- migrations/ — SQL schema for users, assets, balances, quotes, indexes, and triggers (plus default assets like SOL/USDC/USDT).

Environment: .env files exist at root, backend/, indexer/, and sample node envs in mpc/.env.node{1,2,3}. Crates run independently (no workspace at root).

## What’s working (achieved)
- Repo structure and environment layout, with per-crate build/run/test commands
- Database migrations:
  - Tables: users (with public_key), assets, balances (unique user_id, asset_id), quotes (quote_data JSONB, used, expires_at)
  - Indexes for core lookups; update triggers for updated_at
  - Default assets seeded (SOL/USDC/USDT)
- Store crate:
  - Users: create/authenticate/verify/update public key/delete/list/stats
  - Balances: get/update/increment; get-or-create asset inside txn; by-mint queries
  - Quotes: create/get/get_valid/mark_used/cleanup_expired
- Backend:
  - JwtAuth middleware with public-route bypass and helpful error responses
  - Routes: /api/user/signup, /api/user/signin, /api/user/profile; /health
  - Services: MpcClient (mock mode works today), JupiterClient (mock quotes and transactions), Solana client (real builders but mock mode enabled in main)
- MPC node:
  - HTTP server with endpoints for keygen and 2-phase signing
  - FROST-based dealer keygen (POC) and sled-backed persistence
  - Health endpoint and basic serialization types
- Indexer:
  - Compiles in test profile; lightweight unit tests pass

## What’s partially implemented
- Real MPC signing path across backend and MPC nodes:
  - Backend MpcClient mock mode works; real mode endpoint names don’t yet align (backend expects /api/sign; nodes expose /api/sign-phase1 and /api/sign-phase2)
- Solana integration:
  - Real building/broadcast helpers exist; backend is hard-coded to mock mode
- Jupiter integration:
  - Mock quotes and mock swap transaction generation; real wire-up TBD
- Indexer processing:
  - Types and DB helpers exist; end-to-end geyser subscription + DB update loop incomplete

## What’s missing or blocking
- Backend build conflict (dependency versions):
  - spl-associated-token-account (via spl-token-2022 → curve25519-dalek v3) pulls zeroize <1.4
  - bcrypt pulls zeroize >=1.5
  - Result: backend fails to compile by default. Recommended: feature-gate real_solana deps (spl-token/spl-ata) so default mock build doesn’t pull conflicting versions; or upgrade Solana deps to versions compatible with curve25519-dalek v4 and a zeroize range that coexists with bcrypt.
- SQLx offline cache not prepared in store/indexer:
  - Builds/tests require a live DATABASE_URL unless `cargo sqlx prepare` is run to cache queries
- Duplicate DB logic in backend/src/store.rs vs store crate:
  - Prefer consolidating all DB access in the store crate and removing backend-local duplication
- MPC key shares in central DB:
  - The root migration includes a keyshares table with a private_key column. For production MPC, key material should remain on MPC nodes only (sled), not in a central database
- End-to-end transaction signing and swap execution (backend↔mpc↔solana/jupiter) not yet wired in real mode
- Indexer integration not yet feeding backend balance APIs (backend currently leans on RPC or mock)
- Production hardening: rate limiting, inter-node auth, improved error handling/retries, metrics/health, etc.

## Quick status by plan phases
- Phase 1 (Core Infra)
  - Store operations: largely complete
  - DB schema: present and solid; indexes reasonable; migrations ready
  - Success criteria: Requires DB up + sqlx prepare to run tests and confirm
- Phase 2 (MPC)
  - MPC node: endpoints + sled persistence; FROST dealer DKG; health OK
  - Integration: real signing flow not aligned with backend client yet
- Phase 3 (Backend)
  - Auth and routes implemented; mock MPC/Jupiter working; Solana client in mock mode
  - Blocked by dependency conflict for a real-mode build
- Phase 4 (Solana)
  - Transaction builders/helpers implemented; still in mock mode operationally
- Phase 5 (Jupiter)
  - Mock client usable for quote/swap scaffolding; needs real API wiring + store integration for quotes
- Phase 6 (Indexer)
  - Skeleton + DB helpers + tests; end-to-end streaming/processing not complete
- Phase 7 (Production)
  - Not started; items identified and planned

## Notable gotchas and risks
- Dependency conflict (zeroize) blocks backend build when Solana token crates are enabled
- Endpoint mismatch between backend and MPC node for real signing flow
- SQLx requires either live DB or prepared cache to compile/test store/indexer
- Centralized storage of key shares is a security smell for MPC (keep in node-local sled)
- Some duplication between backend-local store module and the store crate

## Suggested next steps
1) Unblock backend builds: feature-gate Solana token deps (default off) and guard code with cfg(feature) while mock mode is on
2) Stand up Postgres, run migrations, and run `cargo sqlx prepare` for store and indexer; then run their tests
3) Align backend MpcClient with MPC node’s two-phase endpoints (or add a coordinator endpoint on nodes)
4) Remove backend/src/store.rs duplication and rely on store crate exclusively
5) Remove centralized keyshares private data from DB; rely on sled per node
6) Implement real Jupiter quote/swap requests; store quotes on receipt; wire swap execution through MPC signing and Solana broadcast
7) Finish indexer loop and swap backend balance RPCs for indexed reads
8) Add production hardening (rate limiting, inter-node auth, retries/circuit breakers, observability)

## Fast reference (current behavior)
- Backend defaults to mock clients for MPC, Jupiter, and Solana, enabling local API iteration without external deps
- MPC node can be launched per sample envs; endpoints exist for keygen/signing phases; persistence via sled;
- Store/indexer require live DB or SQLx prepare for compilation/tests

