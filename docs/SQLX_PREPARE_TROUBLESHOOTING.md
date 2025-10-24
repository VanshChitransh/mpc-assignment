# SQLx Prepare Troubleshooting Guide

**Complete reference for all Phase 0 indexer SQLx prepare blockers and fixes**

---

## Overview

This document captures every blocker encountered during `cargo sqlx prepare` for the indexer crate, with exact fixes and explanations.

---

## Blocker 1: Permission Denied for Tables

### Symptom
```bash
cargo sqlx prepare
# Error: permission denied for table user_wallets, indexer_state, balance_changes, token_balances, users, etc.
```

### Root Cause
The role used during prepare (typically `postgres`) lacks privileges on objects in the `solana_indexer` database. This happens when:
- The database/schema is owned by a different role
- Migrations were run as a different user
- Default privileges weren't set properly

### Fix (Choose ONE)

#### Option A: Prepare as Database Owner
```bash
# If database is owned by different user (e.g., 'indexer_user')
DATABASE_URL=postgresql://indexer_user:password@localhost:5432/solana_indexer cargo sqlx prepare
```

#### Option B: Grant Privileges to postgres ⭐ RECOMMENDED
```bash
# Connect to database as owner or superuser
docker exec -it solana-wallet-db psql -U postgres -d solana_indexer

# Then run:
GRANT USAGE ON SCHEMA public TO postgres;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO postgres;
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO postgres;
```

After granting:
```bash
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer cargo sqlx prepare
```

---

## Blocker 2: Relation Does Not Exist

### Symptom
```bash
cargo sqlx prepare
# Error: relation "user_wallets" does not exist
# Error: relation "indexer_state" does not exist
```

### Root Cause
Using the wrong database. Tables like `user_wallets`, `indexer_state` exist in `solana_indexer`, not in `solana_wallet`.

### Fix
```bash
# Always use solana_indexer for indexer crate
cd indexer
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer cargo sqlx prepare
```

**Also ensure `indexer/.env` points to correct database:**
```env
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer
```

---

## Blocker 3: Permission Denied for Schema Public

### Symptom
```bash
sqlx migrate run
# Error: permission denied for schema public
```

### Root Cause
Schema or database owned by different role; postgres has no DDL rights.

### Fix (Choose ONE)

#### Option A: Run Migrations as Owner
```bash
DATABASE_URL=postgresql://indexer_owner:password@localhost:5432/solana_indexer sqlx migrate run --source ./migrations
```

#### Option B: Transfer Ownership to postgres ⭐ RECOMMENDED
```bash
# Connect as superuser
docker exec -it solana-wallet-db psql -U postgres

# Transfer ownership
ALTER DATABASE solana_indexer OWNER TO postgres;
\c solana_indexer
ALTER SCHEMA public OWNER TO postgres;
```

Then run migrations:
```bash
cd indexer
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer sqlx migrate run
```

---

## Blocker 4: Migration Was Previously Applied But Is Missing

### Symptom
```bash
sqlx migrate run
# Error: migration 2 was previously applied but is missing in the resolved migrations
```

### Root Cause
Mixing root and indexer migrations in the same database. Both write to `_sqlx_migrations` table, creating history conflicts.

### Fix: Use Separate Databases ⭐ BEST PRACTICE

**Correct setup:**
- `solana_wallet` - for backend/store crate migrations (root `/migrations`)
- `solana_indexer` - for indexer crate migrations (`indexer/migrations`)

```bash
# Backend/store migrations
cd /purge-assignment
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_wallet sqlx migrate run

# Indexer migrations (separate DB)
cd indexer
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer sqlx migrate run
```

**Alternative (not recommended):** If you must share a database, manually clean conflicting rows in `_sqlx_migrations`, but this is risky.

---

## Blocker 5: Permission Denied to Create Database

### Symptom
```bash
sqlx database create
# Error: permission denied to create database
```

### Root Cause
The connecting role lacks `CREATEDB` privileges.

### Fix
```bash
# Create database via docker exec as superuser
docker exec -it solana-wallet-db psql -U postgres -c 'CREATE DATABASE solana_indexer;'

# Or grant CREATEDB to role
docker exec -it solana-wallet-db psql -U postgres -c 'ALTER ROLE postgres CREATEDB;'
```

---

## Blocker 6: sqlx-cli vs Project Version Mismatch

### Symptom
Odd validation/prepare behavior, inconsistent errors between CLI and compile-time checks.

### Root Cause
Using sqlx-cli 0.8.x against project using sqlx 0.7.x.

### Fix: Match CLI Version to Project ⭐ IMPORTANT
```bash
# Check project's sqlx version
grep 'sqlx =' Cargo.toml
# Example: sqlx = { version = "0.7", ... }

# Install matching CLI version
cargo install sqlx-cli --version 0.7.4 --no-default-features --features native-tls,postgres --force

# Verify
sqlx --version
# Should show: sqlx-cli 0.7.4
```

---

## Blocker 7: macOS TLS/Permissions Quirks

### Symptom
Environment-specific failures even with correct DB and privileges. May manifest as:
- Connection timeouts during prepare
- TLS handshake failures
- Unexplained permission errors

### Fix (Choose ONE)

#### Option A: Use native-tls ⭐ FASTEST
```bash
cargo install sqlx-cli --version 0.7.4 --no-default-features --features native-tls,postgres --force
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer cargo sqlx prepare
```

#### Option B: Run Prepare in Linux Container
```bash
docker run --rm \
  -v "$PWD":/app -w /app/indexer \
  -e DATABASE_URL=postgresql://postgres:postgres@host.docker.internal:5432/solana_indexer \
  rust:1.79-bullseye bash -lc '
    cargo install sqlx-cli --version 0.7.4 --no-default-features --features rustls,postgres --force && \
    cargo sqlx prepare
  '
```

---

## Blocker 8: Extra Binaries Compiled During Prepare

### Symptom
```bash
cargo sqlx prepare
# Errors from src/bin/test_client.rs during prepare
# Additional compilation time/failures
```

### Root Cause
Test binaries being compiled during prepare, adding compilation surface and potential failures.

### Fix: Disable Auto-binaries ⭐ ALREADY APPLIED
```toml
# indexer/Cargo.toml
[package]
autobins = false  # ✅ Already set

# Only explicitly declared binaries will compile
[[bin]]
name = "indexer"
path = "src/main.rs"

# Comment out test binaries
# [[bin]]
# name = "test_client"
# path = "src/bin/test_client.rs"
```

---

## Recommended Path to Complete Phase 0

### Quick Fix (90% of cases)

1. **Ensure correct database and permissions:**
```bash
# Check database ownership
docker exec -it solana-wallet-db psql -U postgres -d solana_indexer -c "\l solana_indexer"
docker exec -it solana-wallet-db psql -U postgres -d solana_indexer -c "\dt"

# If needed, grant permissions (run migration 002_grant_permissions.sql)
docker exec -it solana-wallet-db psql -U postgres -d solana_indexer -f /path/to/indexer/migrations/002_grant_permissions.sql
```

2. **Use correct sqlx-cli version with native-tls:**
```bash
cargo install sqlx-cli --version 0.7.4 --no-default-features --features native-tls,postgres --force
```

3. **Run prepare with correct DATABASE_URL:**
```bash
cd indexer
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer cargo sqlx prepare
```

4. **Commit the generated cache:**
```bash
git add indexer/.sqlx/
git commit -m "Add indexer SQLx offline cache"
```

### If macOS Quirks Persist

Run containerized prepare (see Blocker 7 Option B), then commit `indexer/sqlx-data.json` so future builds can use `SQLX_OFFLINE=true`.

---

## Verification Checklist

- [ ] Docker container `solana-wallet-db` is running
- [ ] Database `solana_indexer` exists
- [ ] Migrations applied: `docker exec -it solana-wallet-db psql -U postgres -d solana_indexer -c "SELECT * FROM _sqlx_migrations;"`
- [ ] Tables exist: `docker exec -it solana-wallet-db psql -U postgres -d solana_indexer -c "\dt"`
- [ ] Permissions granted: Check `002_grant_permissions.sql` applied
- [ ] sqlx-cli version matches project: `sqlx --version` shows 0.7.4
- [ ] `indexer/.env` points to `solana_indexer` database
- [ ] `autobins = false` in `indexer/Cargo.toml`

---

## Common Pitfalls

1. **Using wrong database:** Always use `solana_indexer` for indexer, not `solana_wallet`
2. **sqlx version mismatch:** CLI must match project version (0.7.x)
3. **Mixed migrations:** Keep indexer migrations separate from backend migrations
4. **Insufficient permissions:** postgres needs at least SELECT on all tables for prepare
5. **macOS TLS:** Use native-tls instead of rustls for prepare on macOS

---

## Success Indicators

When prepare succeeds, you'll see:
```bash
cargo sqlx prepare
# Output:
#   Compiling sqlx-macros v0.7.4
#   Compiling solana-wallet-indexer v0.1.0
# query data written to `.sqlx` in the current directory; please check this into version control
```

Check:
```bash
ls -la indexer/.sqlx/
# Should contain query-*.json files
```

---

## Next Steps After Successful Prepare

1. **Enable offline mode:**
```bash
# In CI or when DB not available
SQLX_OFFLINE=true cargo build
```

2. **Keep cache updated:**
```bash
# After changing queries
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer cargo sqlx prepare
git add indexer/.sqlx/
git commit -m "Update SQLx offline cache"
```

3. **Proceed to Phase 1** with confidence that indexer can build offline.

---

## Reference: Working Setup

```bash
# Environment
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer

# Database state
docker exec -it solana-wallet-db psql -U postgres -d solana_indexer -c "\dt"
#  user_wallets, balance_changes, token_balances, transactions, indexer_state, etc.

# Permissions
docker exec -it solana-wallet-db psql -U postgres -d solana_indexer -c "\du postgres"
#  postgres | Superuser, Create role, Create DB

# SQLx CLI
sqlx --version
#  sqlx-cli 0.7.4

# Prepare command
cd indexer
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer cargo sqlx prepare
```

---

**Document version:** 1.0  
**Last updated:** October 16, 2025  
**Status:** Complete reference for all known Phase 0 indexer blockers

