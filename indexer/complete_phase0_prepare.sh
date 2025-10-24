#!/bin/bash
# Complete Phase 0 - SQLx Prepare for Indexer
# Addresses all documented blockers from SQLX_PREPARE_TROUBLESHOOTING.md

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_header() {
    echo -e "\n${GREEN}=== $1 ===${NC}\n"
}

# Change to indexer directory
cd "$(dirname "$0")"

print_header "Phase 0 Indexer SQLx Prepare - Complete Fix"

# Step 1: Verify Docker is running
print_status "Checking Docker..."
if ! docker ps &> /dev/null; then
    print_error "Docker is not running. Please start Docker Desktop and try again."
    exit 1
fi
print_status "Docker is running"

# Step 2: Verify PostgreSQL container
print_status "Checking PostgreSQL container..."
if ! docker ps | grep -q solana-wallet-db; then
    print_error "PostgreSQL container 'solana-wallet-db' is not running."
    echo "Start it with: docker start solana-wallet-db"
    echo "Or create it with: docker run -d --name solana-wallet-db -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:15"
    exit 1
fi
print_status "PostgreSQL container is running"

# Step 3: Verify database exists
print_status "Checking solana_indexer database..."
if ! docker exec solana-wallet-db psql -U postgres -lqt | cut -d \| -f 1 | grep -qw solana_indexer; then
    print_warning "Database 'solana_indexer' doesn't exist. Creating it..."
    docker exec solana-wallet-db psql -U postgres -c "CREATE DATABASE solana_indexer;"
    print_status "Database created"
else
    print_status "Database exists"
fi

# Step 4: Check if migrations are applied
print_status "Checking migrations..."
MIGRATION_COUNT=$(docker exec solana-wallet-db psql -U postgres -d solana_indexer -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '_sqlx_migrations';" 2>/dev/null || echo "0")

if [ "$MIGRATION_COUNT" = "0" ]; then
    print_warning "Migrations not applied. Applying now..."
    
    # Apply migrations manually
    for migration in migrations/*.sql; do
        if [ -f "$migration" ]; then
            print_status "Applying $(basename "$migration")..."
            docker exec -i solana-wallet-db psql -U postgres -d solana_indexer < "$migration"
        fi
    done
    print_status "Migrations applied"
else
    print_status "Migrations table exists"
fi

# Step 5: Verify tables exist
print_status "Verifying tables..."
TABLES=$(docker exec solana-wallet-db psql -U postgres -d solana_indexer -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE';" 2>/dev/null || echo "0")

if [ "$TABLES" -lt 5 ]; then
    print_error "Expected at least 5 tables, found $TABLES. Migration may have failed."
    exit 1
fi
print_status "Found $TABLES tables in database"

# Step 6: Apply permission grants
print_status "Ensuring permissions are set..."
docker exec solana-wallet-db psql -U postgres -d solana_indexer <<EOF
GRANT USAGE ON SCHEMA public TO postgres;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO postgres;
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO postgres;
EOF
print_status "Permissions granted"

# Step 7: Verify sqlx-cli version
print_status "Checking sqlx-cli version..."
if ! command -v sqlx &> /dev/null; then
    print_warning "sqlx-cli not installed. Installing version 0.7.4..."
    cargo install sqlx-cli --version 0.7.4 --no-default-features --features native-tls,postgres --force
else
    SQLX_VERSION=$(sqlx --version | grep -oE '[0-9]+\.[0-9]+' | head -1)
    if [[ ! "$SQLX_VERSION" =~ ^0\.7 ]]; then
        print_warning "sqlx-cli version $SQLX_VERSION doesn't match project (0.7.x). Installing 0.7.4..."
        cargo install sqlx-cli --version 0.7.4 --no-default-features --features native-tls,postgres --force
    else
        print_status "sqlx-cli version $SQLX_VERSION is compatible"
    fi
fi

# Step 8: Create/update .env file
print_status "Ensuring .env file is configured..."
cat > .env <<EOF
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/solana_indexer
RUST_LOG=info
EOF
print_status ".env file configured"

# Step 9: Run cargo sqlx prepare
print_header "Running cargo sqlx prepare"
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/solana_indexer"

if cargo sqlx prepare 2>&1 | tee /tmp/sqlx_prepare.log; then
    print_status "✅ cargo sqlx prepare completed successfully!"
    
    # Verify .sqlx directory was created
    if [ -d ".sqlx" ]; then
        QUERY_COUNT=$(find .sqlx -name "query-*.json" | wc -l)
        print_status "Generated .sqlx cache with $QUERY_COUNT query files"
        
        print_header "Phase 0 Complete!"
        echo "✅ All blockers resolved"
        echo "✅ SQLx offline cache generated"
        echo "✅ Indexer can now build with SQLX_OFFLINE=true"
        echo ""
        echo "Next steps:"
        echo "  1. Commit the cache: git add .sqlx/ && git commit -m 'Add indexer SQLx offline cache'"
        echo "  2. Proceed to Phase 1"
    else
        print_warning "Prepare succeeded but .sqlx directory not found"
    fi
else
    print_error "cargo sqlx prepare failed. See /tmp/sqlx_prepare.log for details"
    echo ""
    echo "Common fixes:"
    echo "  1. Check database connection: psql -U postgres -h localhost -d solana_indexer"
    echo "  2. Verify tables exist: docker exec solana-wallet-db psql -U postgres -d solana_indexer -c '\dt'"
    echo "  3. Try containerized prepare (see docs/SQLX_PREPARE_TROUBLESHOOTING.md Blocker 7)"
    echo ""
    echo "Full troubleshooting guide: docs/SQLX_PREPARE_TROUBLESHOOTING.md"
    exit 1
fi

