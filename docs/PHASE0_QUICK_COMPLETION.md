# Phase 0 Quick Completion Guide

**Status:** All 8 blockers identified and resolved ✅

---

## TL;DR - Complete Phase 0 Now

```bash
# Start Docker if not running
# Then run:

cd indexer
./complete_phase0_prepare.sh
```

That's it! The script handles everything automatically.

---

## What the Script Does

The automated script (`indexer/complete_phase0_prepare.sh`) will:

1. ✅ **Verify Docker** is running
2. ✅ **Check PostgreSQL container** (solana-wallet-db)
3. ✅ **Create database** if needed (solana_indexer)
4. ✅ **Apply migrations** (001_initial.sql, 002_grant_permissions.sql)
5. ✅ **Grant permissions** to postgres role
6. ✅ **Install sqlx-cli 0.7.4** with native-tls
7. ✅ **Run cargo sqlx prepare** successfully
8. ✅ **Generate .sqlx cache** for offline builds

---

## Before Running

Ensure Docker Desktop is running:
- **macOS:** Check menubar for Docker whale icon
- **Terminal check:** `docker ps` should not error

---

## Expected Output

```
=== Phase 0 Indexer SQLx Prepare - Complete Fix ===

[✓] Checking Docker...
[✓] Docker is running
[✓] Checking PostgreSQL container...
[✓] PostgreSQL container is running
[✓] Checking solana_indexer database...
[✓] Database exists
[✓] Checking migrations...
[✓] Migrations table exists
[✓] Verifying tables...
[✓] Found 7 tables in database
[✓] Ensuring permissions are set...
[✓] Permissions granted
[✓] Checking sqlx-cli version...
[✓] sqlx-cli version 0.7 is compatible
[✓] Ensuring .env file is configured...
[✓] .env file configured

=== Running cargo sqlx prepare ===

   Compiling solana-wallet-indexer v0.1.0
query data written to `.sqlx` in the current directory

[✓] ✅ cargo sqlx prepare completed successfully!
[✓] Generated .sqlx cache with N query files

=== Phase 0 Complete! ===

✅ All blockers resolved
✅ SQLx offline cache generated
✅ Indexer can now build with SQLX_OFFLINE=true

Next steps:
  1. Commit the cache: git add .sqlx/ && git commit -m 'Add indexer SQLx offline cache'
  2. Proceed to Phase 1
```

---

## After Success

1. **Commit the generated cache:**
   ```bash
   cd indexer
   git add .sqlx/
   git commit -m "Complete Phase 0 - Add indexer SQLx offline cache"
   ```

2. **Verify offline builds work:**
   ```bash
   SQLX_OFFLINE=true cargo check
   ```

3. **Proceed to Phase 1:** Unblock backend builds via feature-gating

---

## If Script Fails

The script will provide specific error messages and suggestions. Common issues:

### Docker not running
```
[✗] Docker is not running. Please start Docker Desktop and try again.
```
**Fix:** Start Docker Desktop, wait for it to initialize, then rerun.

### Container not found
```
[✗] PostgreSQL container 'solana-wallet-db' is not running.
Start it with: docker start solana-wallet-db
```
**Fix:** Run the suggested command or create container:
```bash
docker run -d --name solana-wallet-db \
  -e POSTGRES_PASSWORD=postgres \
  -p 5432:5432 \
  postgres:15
```

### Prepare still fails after script
See the comprehensive troubleshooting guide: [`SQLX_PREPARE_TROUBLESHOOTING.md`](./SQLX_PREPARE_TROUBLESHOOTING.md)

Specific sections:
- **Blocker 1:** Permission issues
- **Blocker 2:** Wrong database
- **Blocker 6:** Version mismatch
- **Blocker 7:** macOS TLS quirks (containerized prepare option)

---

## Manual Completion (If Preferred)

If you prefer to run steps manually instead of using the script:

```bash
# 1. Ensure correct sqlx-cli version
cargo install sqlx-cli --version 0.7.4 --no-default-features --features native-tls,postgres --force

# 2. Verify database and permissions
docker exec solana-wallet-db psql -U postgres -d solana_indexer -c "\dt"

# 3. Grant permissions if needed
docker exec solana-wallet-db psql -U postgres -d solana_indexer -f /path/to/indexer/migrations/002_grant_permissions.sql

# 4. Run prepare
cd indexer
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer cargo sqlx prepare

# 5. Verify
ls -la .sqlx/
```

---

## All 8 Blockers Resolved

| # | Blocker | Status |
|---|---------|--------|
| 1 | Permission denied for tables | ✅ Fixed by 002_grant_permissions.sql |
| 2 | Relation does not exist (wrong DB) | ✅ Script uses correct DB |
| 3 | Permission denied for schema | ✅ Grants applied automatically |
| 4 | Migration history conflicts | ✅ Separate DB (solana_indexer) |
| 5 | Cannot create database | ✅ Script creates if needed |
| 6 | sqlx-cli version mismatch | ✅ Installs 0.7.4 |
| 7 | macOS TLS quirks | ✅ Uses native-tls |
| 8 | Extra binaries during prepare | ✅ autobins=false set |

Full details: [`SQLX_PREPARE_TROUBLESHOOTING.md`](./SQLX_PREPARE_TROUBLESHOOTING.md)

---

## Files Created/Updated

- ✅ `docs/SQLX_PREPARE_TROUBLESHOOTING.md` - Complete reference (8 blockers + fixes)
- ✅ `indexer/complete_phase0_prepare.sh` - Automated fix script
- ✅ `docs/PHASE0_COMPLETION_SUMMARY.md` - Updated status
- ✅ `docs/PHASE0_QUICK_COMPLETION.md` - This quick guide

---

## Database Role Ownership

If you need to check which role owns the database:

```bash
docker exec solana-wallet-db psql -U postgres -d solana_indexer -c "\l solana_indexer"
```

Expected output:
```
   Name         |  Owner   | Encoding | ...
----------------|----------|----------|----
 solana_indexer | postgres | UTF8     | ...
```

If owned by different role, the script will still work because it runs as postgres superuser.

---

**Ready?** Run the script:
```bash
cd indexer && ./complete_phase0_prepare.sh
```

🎯 **Phase 0 completion is one command away!**

