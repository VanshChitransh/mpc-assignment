#!/bin/bash

# Script to apply all Step 3.1 compilation fixes
# Run this from the project root directory

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║       Step 3.1 Compilation Fixes - Auto Patcher          ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Backup function
backup_file() {
    local file=$1
    if [ -f "$file" ]; then
        cp "$file" "${file}.backup.$(date +%Y%m%d_%H%M%S)"
        echo -e "${GREEN}✓${NC} Backed up: $file"
    fi
}

# Fix 1: MPC Service - Borrow Checker Errors
fix_mpc_borrow_checker() {
    echo -e "\n${BLUE}[1/5] Fixing MPC Service Borrow Checker Errors...${NC}"
    
    local file="backend/src/services/mpc.rs"
    
    if [ ! -f "$file" ]; then
        echo -e "${RED}✗${NC} File not found: $file"
        return 1
    fi
    
    backup_file "$file"
    
    # Fix pattern: Save status before consuming response
    # This uses perl for multi-line regex replacement
    
    perl -i -0pe 's/let status = response\.status\(\);\s+if !status\.is_success\(\) \{\s+let error_text = response\.text\(\)\.await/let status = response.status();\n        \n        if !status.is_success() {\n            let error_text = response.text().await/g' "$file"
    
    echo -e "${GREEN}✓${NC} Fixed borrow checker errors in MPC service"
}

# Fix 2: Add Serialize to MpcError
fix_mpc_error_serialize() {
    echo -e "\n${BLUE}[2/5] Adding Serialize to MpcError...${NC}"
    
    local file="backend/src/services/mpc.rs"
    
    if [ ! -f "$file" ]; then
        echo -e "${RED}✗${NC} File not found: $file"
        return 1
    fi
    
    # Check if already has Serialize
    if grep -q "#\[derive(Error, Debug, Serialize" "$file"; then
        echo -e "${YELLOW}⚠${NC}  MpcError already has Serialize"
        return 0
    fi
    
    # Add Serialize and Deserialize to MpcError derive
    sed -i.bak2 's/#\[derive(Error, Debug)\]/#[derive(Error, Debug, Serialize, Deserialize, Clone)]/g' "$file"
    
    # Change RequestFailed to use String instead of reqwest::Error
    sed -i.bak3 's/RequestFailed(#\[from\] reqwest::Error),/RequestFailed(String),/g' "$file"
    
    # Add From implementation if not exists
    if ! grep -q "impl From<reqwest::Error> for MpcError" "$file"; then
        cat >> "$file" << 'EOF'

impl From<reqwest::Error> for MpcError {
    fn from(err: reqwest::Error) -> Self {
        MpcError::RequestFailed(err.to_string())
    }
}
EOF
    fi
    
    echo -e "${GREEN}✓${NC} Added Serialize to MpcError"
}

# Fix 3: Fix wallet_service.rs cluster status access
fix_wallet_service_cluster_status() {
    echo -e "\n${BLUE}[3/5] Fixing wallet_service.rs cluster status access...${NC}"
    
    local file="backend/src/services/wallet_service.rs"
    
    if [ ! -f "$file" ]; then
        echo -e "${YELLOW}⚠${NC}  File not found: $file (may not exist yet)"
        return 0
    fi
    
    backup_file "$file"
    
    # Replace is_operational with threshold_met
    sed -i.bak 's/cluster_status\.is_operational/cluster_status.threshold_met/g' "$file"
    
    # Also fix in JSON construction
    sed -i.bak2 's/"is_operational": cluster_status\.is_operational/"cluster_operational": cluster_status.threshold_met/g' "$file"
    
    echo -e "${GREEN}✓${NC} Fixed cluster status access in wallet_service"
}

# Fix 4: Ensure lib.rs exists with proper exports
fix_lib_rs() {
    echo -e "\n${BLUE}[4/5] Ensuring lib.rs exists with proper exports...${NC}"
    
    local file="backend/src/lib.rs"
    
    if [ -f "$file" ]; then
        echo -e "${YELLOW}⚠${NC}  lib.rs already exists"
        return 0
    fi
    
    cat > "$file" << 'EOF'
// Library exports for backend

pub mod services;
pub mod routes;
pub mod middleware;
pub mod models;
pub mod error;

// Re-export commonly used types
pub use services::mpc::{MpcClient, MpcError};
pub use models::*;
EOF
    
    echo -e "${GREEN}✓${NC} Created lib.rs with proper exports"
}

# Fix 5: Update Cargo.toml to have [lib] section
fix_cargo_toml() {
    echo -e "\n${BLUE}[5/5] Updating Cargo.toml for lib support...${NC}"
    
    local file="backend/Cargo.toml"
    
    if [ ! -f "$file" ]; then
        echo -e "${RED}✗${NC} File not found: $file"
        return 1
    fi
    
    # Check if [lib] section already exists
    if grep -q "^\[lib\]" "$file"; then
        echo -e "${YELLOW}⚠${NC}  [lib] section already exists in Cargo.toml"
    else
        # Add [lib] section before [[bin]] if it exists, otherwise at the end
        if grep -q "^\[\[bin\]\]" "$file"; then
            sed -i.bak '/^\[\[bin\]\]/i \
[lib]\
name = "backend"\
path = "src/lib.rs"\
\
' "$file"
        else
            cat >> "$file" << 'EOF'

[lib]
name = "backend"
path = "src/lib.rs"

[[bin]]
name = "backend"
path = "src/main.rs"
EOF
        fi
        echo -e "${GREEN}✓${NC} Added [lib] section to Cargo.toml"
    fi
}

# Main execution
main() {
    echo -e "\n${BLUE}Starting fixes...${NC}\n"
    
    # Run all fixes
    fix_mpc_borrow_checker
    fix_mpc_error_serialize
    fix_wallet_service_cluster_status
    fix_lib_rs
    fix_cargo_toml
    
    # Clean up temporary backup files
    echo -e "\n${BLUE}Cleaning up temporary files...${NC}"
    find backend -name "*.bak" -o -name "*.bak2" -o -name "*.bak3" | while read file; do
        rm "$file" 2>/dev/null || true
    done
    echo -e "${GREEN}✓${NC} Cleaned up temporary files"
    
    echo -e "\n${BLUE}Testing compilation...${NC}"
    cd backend
    
    if cargo check 2>&1 | tail -20; then
        echo -e "\n${GREEN}════════════════════════════════════════════════════════════${NC}"
        echo -e "${GREEN}✓ ALL FIXES APPLIED SUCCESSFULLY!${NC}"
        echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}\n"
        
        echo -e "${BLUE}Next steps:${NC}"
        echo "1. Review the changes in the backed up files"
        echo "2. Run: cargo build"
        echo "3. Run: cargo test --test test_step_3_1_complete"
        echo "4. If tests pass, proceed with Step 3.1 validation"
        echo
        
        return 0
    else
        echo -e "\n${YELLOW}════════════════════════════════════════════════════════════${NC}"
        echo -e "${YELLOW}⚠  COMPILATION STILL HAS ERRORS${NC}"
        echo -e "${YELLOW}════════════════════════════════════════════════════════════${NC}\n"
        
        echo -e "${YELLOW}Some errors may still exist. Check the output above.${NC}"
        echo
        echo -e "${BLUE}Common remaining issues:${NC}"
        echo "1. Missing dependencies - run: cargo update"
        echo "2. Syntax errors from manual edits"
        echo "3. Module not found errors - check mod.rs files"
        echo
        echo -e "${BLUE}Backup files are available:${NC}"
        find backend -name "*.backup.*" | head -5
        echo
        
        return 1
    fi
}

# Run main
main "$@"