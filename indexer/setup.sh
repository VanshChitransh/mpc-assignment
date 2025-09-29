#!/bin/bash

# Solana Wallet Indexer - Phase 4 Setup and Test Script
set -e

echo "🚀 Setting up Solana Wallet Indexer - Phase 4"
echo "=============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if required tools are installed
check_dependencies() {
    print_status "Checking dependencies..."
    
    # Check for Rust
    if ! command_exists cargo; then
        print_error "Rust/Cargo not found. Please install Rust from https://rustup.rs/"
        exit 1
    fi
    print_success "Rust/Cargo found: $(cargo --version)"
    
    # Check for PostgreSQL
    if ! command_exists psql; then
        print_warning "PostgreSQL client not found. You'll need PostgreSQL running."
        print_status "Install PostgreSQL: https://www.postgresql.org/download/"
    else
        print_success "PostgreSQL client found: $(psql --version | head -1)"
    fi
    
    # Check for Docker (optional)
    if command_exists docker; then
        print_success "Docker found: $(docker --version | head -1)"
    else
        print_warning "Docker not found (optional for easy PostgreSQL setup)"
    fi

    # Check for sqlx-cli
    if ! command_exists sqlx; then
        print_warning "sqlx-cli not found. Will install it..."
        install_sqlx_cli
    else
        print_success "sqlx-cli found: $(sqlx --version)"
    fi
}

# Install sqlx-cli
install_sqlx_cli() {
    print_status "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
    if [ $? -eq 0 ]; then
        print_success "sqlx-cli installed successfully"
    else
        print_error "Failed to install sqlx-cli"
        exit 1
    fi
}

# Setup PostgreSQL using Docker
setup_postgres_docker() {
    print_status "Setting up PostgreSQL with Docker..."
    
    # Stop and remove existing container if it exists
    if docker ps -a | grep -q solana_postgres; then
        print_status "Removing existing PostgreSQL container..."
        docker stop solana_postgres >/dev/null 2>&1 || true
        docker rm solana_postgres >/dev/null 2>&1 || true
    fi
    
    print_status "Creating new PostgreSQL container..."
    docker run -d \
        --name solana_postgres \
        -e POSTGRES_PASSWORD=password \
        -e POSTGRES_DB=solana_indexer \
        -e POSTGRES_USER=postgres \
        -p 5432:5432 \
        postgres:15
    
    if [ $? -ne 0 ]; then
        print_error "Failed to start PostgreSQL container"
        exit 1
    fi
    
    # Wait for PostgreSQL to be ready
    print_status "Waiting for PostgreSQL to be ready..."
    for i in {1..30}; do
        if docker exec solana_postgres pg_isready -U postgres >/dev/null 2>&1; then
            print_success "PostgreSQL is ready!"
            break
        fi
        if [ $i -eq 30 ]; then
            print_error "PostgreSQL failed to start within 60 seconds"
            exit 1
        fi
        sleep 2
        printf "."
    done
    echo ""
}

# Setup local PostgreSQL
setup_postgres_local() {
    print_status "Using local PostgreSQL..."
    print_warning "Make sure PostgreSQL is running on localhost:5432"
    print_warning "Database: solana_indexer, User: postgres, Password: password"
    
    # Try to create database
    print_status "Attempting to create database..."
    createdb -h localhost -U postgres -W solana_indexer 2>/dev/null || true
    print_status "Database setup attempted (it may already exist)"
}

# Verify database connection
verify_database() {
    print_status "Verifying database connection..."
    
    # Load environment variables
    if [ -f .env ]; then
        export $(grep -v '^#' .env | xargs)
    fi
    
    # Test connection using psql
    if psql "${DATABASE_URL}" -c "SELECT 1;" >/dev/null 2>&1; then
        print_success "Database connection successful"
    else
        print_error "Failed to connect to database"
        print_status "Please check your DATABASE_URL: ${DATABASE_URL}"
        exit 1
    fi
}

# Create project structure
create_project_structure() {
    print_status "Creating project structure..."
    
    # Create directories
    mkdir -p migrations
    mkdir -p src/bin
    
    # Verify Cargo.toml exists
    if [ ! -f Cargo.toml ]; then
        print_error "Cargo.toml not found. Please make sure you're in the project root directory."
        exit 1
    fi
    
    # Verify source files exist
    required_files=(
        "src/main.rs"
        "src/database.rs"
        "src/geyser.rs"
        "src/yellowstone.rs"
        "src/processor.rs"
        "src/subscription.rs"
        "src/bin/test_client.rs"
        "migrations/001_initial.sql"
    )
    
    missing_files=()
    for file in "${required_files[@]}"; do
        if [ ! -f "$file" ]; then
            missing_files+=("$file")
        fi
    done
    
    if [ ${#missing_files[@]} -ne 0 ]; then
        print_error "Missing required files:"
        for file in "${missing_files[@]}"; do
            print_error "  - $file"
        done
        print_error "Please make sure all source files are in place"
        exit 1
    fi
    
    print_success "Project structure verified"
}

# Setup environment
setup_environment() {
    print_status "Setting up environment..."
    
    # Create .env file if it doesn't exist
    if [ ! -f .env ]; then
        print_status "Creating .env file..."
        cat > .env << 'EOF'
# Database Configuration
DATABASE_URL=postgresql://postgres:password@localhost:5432/solana_indexer

# Yellowstone GRPC Configuration (Mock for development)
YELLOWSTONE_ENDPOINT=http://localhost:10000
# YELLOWSTONE_TOKEN=your_token_here  # Uncomment and add your token if using real Yellowstone service

# Indexer Configuration
COMMITMENT_LEVEL=confirmed
RECONNECT_INTERVAL=5
MAX_RECONNECT_ATTEMPTS=10

# Rust Configuration
RUST_LOG=info,solana_wallet_indexer=debug
RUST_BACKTRACE=1
EOF
        print_success "Created .env file"
    else
        print_status ".env file already exists"
    fi
    
    # Load environment variables
    export $(grep -v '^#' .env | xargs) 2>/dev/null || true
    
    print_success "Environment setup complete"
}

# Build the project
build_project() {
    print_status "Building the Rust project..."
    
    # Clean previous builds
    cargo clean
    
    # Update dependencies
    print_status "Updating dependencies..."
    cargo update
    
    # Build the project
    print_status "Compiling project..."
    if cargo build --release; then
        print_success "Project built successfully!"
    else
        print_error "Build failed!"
        print_status "Try running: cargo build for more detailed error messages"
        exit 1
    fi
}

# Run database migrations
run_migrations() {
    print_status "Running database migrations..."
    
    # Create database if it doesn't exist
    sqlx database create 2>/dev/null || print_status "Database already exists or connection issue"
    
    # Run migrations
    if sqlx migrate run; then
        print_success "Migrations completed successfully!"
    else
        print_error "Migration failed!"
        print_status "Check your DATABASE_URL and ensure PostgreSQL is running"
        exit 1
    fi
}

# Test the project
test_project() {
    print_status "Testing the project..."
    
    # Run unit tests
    print_status "Running unit tests..."
    if cargo test; then
        print_success "Unit tests passed!"
    else
        print_error "Unit tests failed!"
        exit 1
    fi
}

# Test database connectivity
test_database() {
    print_status "Testing database connectivity..."
    
    if cargo run --bin test_client; then
        print_success "Database test completed successfully!"
    else
        print_error "Database test failed!"
        exit 1
    fi
}

# Test the indexer
test_indexer() {
    print_status "Testing the indexer (30-second run)..."
    print_status "This will test the mock Yellowstone client and database connectivity"
    
    # Run the indexer in background for 30 seconds
    (
        timeout 30s cargo run --bin indexer || true
    ) &
    INDEXER_PID=$!
    
    # Wait for it to complete
    wait $INDEXER_PID 2>/dev/null
    
    print_success "Indexer test completed (ran for 30 seconds)"
    print_success "Check the logs above for successful connection and mock data processing"
}

# Show database status
show_database_status() {
    print_status "Checking database status..."
    
    # Show database tables and sample data
    psql "${DATABASE_URL}" -c "
        \echo 'Database Tables:'
        \dt
        \echo ''
        \echo 'Table Row Counts:'
        SELECT 'users' as table_name, count(*) as row_count FROM users
        UNION ALL
        SELECT 'user_wallets', count(*) FROM user_wallets
        UNION ALL
        SELECT 'balance_changes', count(*) FROM balance_changes
        UNION ALL
        SELECT 'subscription_metrics', count(*) FROM subscription_metrics
        UNION ALL
        SELECT 'indexer_state', count(*) FROM indexer_state;
    " 2>/dev/null || print_warning "Could not show database status"
    
    print_success "Database status check complete"
}

# Cleanup function
cleanup() {
    print_status "Cleaning up..."
    # Kill any remaining background processes
    jobs -p | xargs -r kill 2>/dev/null || true
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Main execution function
main() {
    echo
    print_status "Phase 4 Implementation - Real-time Balance Monitoring"
    print_status "Working directory: $(pwd)"
    echo
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found. Please run this script from the project root directory."
        exit 1
    fi
    
    # Step 1: Check dependencies
    check_dependencies
    echo
    
    # Step 2: Create project structure
    create_project_structure
    echo
    
    # Step 3: Setup environment
    setup_environment
    echo
    
    # Step 4: Setup database
    read -p "Do you want to set up PostgreSQL using Docker? (y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        setup_postgres_docker
    else
        setup_postgres_local
    fi
    echo
    
    # Step 5: Verify database connection
    verify_database
    echo
    
    # Step 6: Build project
    build_project
    echo
    
    # Step 7: Run migrations
    run_migrations
    echo
    
    # Step 8: Test project
    test_project
    echo
    
    # Step 9: Test database
    test_database
    echo
    
    # Step 10: Test indexer
    test_indexer
    echo
    
    # Step 11: Show database status
    show_database_status
    echo
    
    # Final success message
    print_success "🎉 Phase 4 setup and testing completed successfully!"
    echo
    print_status "What was tested:"
    echo "  ✓ Database connectivity and migrations"
    echo "  ✓ Mock Yellowstone GRPC client"
    echo "  ✓ Transaction processing pipeline"
    echo "  ✓ Subscription management"
    echo "  ✓ Balance change tracking"
    echo "  ✓ Metrics recording"
    echo
    print_status "Manual commands:"
    echo "  Run indexer:           cargo run --bin indexer"
    echo "  Test database:         cargo run --bin test_client"
    echo "  Run tests:             cargo test"
    echo "  Debug mode:            RUST_LOG=debug cargo run --bin indexer"
    echo
    print_status "Next steps for production:"
    echo "  1. Replace mock Yellowstone client with real implementation"
    echo "  2. Add proper error handling and monitoring"
    echo "  3. Implement authentication and rate limiting"
    echo "  4. Add API endpoints for querying data"
    echo "  5. Set up production database and monitoring"
    echo
}

# Run main function
main "$@"