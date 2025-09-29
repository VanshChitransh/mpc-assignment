#!/bin/bash

# Fix workspace configuration for the indexer project

echo "🔧 Fixing Cargo workspace configuration..."

# Check if parent Cargo.toml exists
PARENT_CARGO="../Cargo.toml"
INDEXER_CARGO="./Cargo.toml"

if [ -f "$PARENT_CARGO" ]; then
    echo "[INFO] Found parent Cargo.toml at $PARENT_CARGO"
    
    # Check if workspace.members exists
    if grep -q "^\[workspace\]" "$PARENT_CARGO"; then
        echo "[INFO] Workspace configuration found"
        
        # Check if members array exists
        if grep -q "members.*=" "$PARENT_CARGO"; then
            # Add indexer to existing members array
            echo "[INFO] Adding 'indexer' to workspace members..."
            
            # Create a backup
            cp "$PARENT_CARGO" "${PARENT_CARGO}.backup"
            
            # Add indexer to members array if not already present
            if ! grep -q '"indexer"' "$PARENT_CARGO"; then
                # Use sed to add indexer to the members array
                sed -i.tmp '/members.*=.*\[/,/\]/ {
                    /\]/ i\
                    "indexer",
                }' "$PARENT_CARGO"
                rm "${PARENT_CARGO}.tmp" 2>/dev/null
                echo "[SUCCESS] Added indexer to workspace members"
            else
                echo "[INFO] indexer already in workspace members"
            fi
        else
            # Add members array to existing workspace
            echo "[INFO] Adding members array to workspace..."
            cp "$PARENT_CARGO" "${PARENT_CARGO}.backup"
            sed -i.tmp '/^\[workspace\]/a\
members = [\
    "indexer",\
]' "$PARENT_CARGO"
            rm "${PARENT_CARGO}.tmp" 2>/dev/null
            echo "[SUCCESS] Added members array with indexer"
        fi
    else
        # Create workspace section
        echo "[INFO] Creating workspace configuration..."
        cp "$PARENT_CARGO" "${PARENT_CARGO}.backup"
        echo "" >> "$PARENT_CARGO"
        echo "[workspace]" >> "$PARENT_CARGO"
        echo "members = [" >> "$PARENT_CARGO"
        echo '    "indexer",' >> "$PARENT_CARGO"
        echo "]" >> "$PARENT_CARGO"
        echo "[SUCCESS] Created workspace with indexer"
    fi
else
    echo "[INFO] No parent Cargo.toml found, making indexer independent..."
    
    # Add empty workspace to indexer Cargo.toml
    if [ -f "$INDEXER_CARGO" ]; then
        if ! grep -q "^\[workspace\]" "$INDEXER_CARGO"; then
            cp "$INDEXER_CARGO" "${INDEXER_CARGO}.backup"
            
            # Add empty workspace at the top of the file
            echo -e "[workspace]\n# This makes the package independent\n\n$(cat $INDEXER_CARGO)" > "$INDEXER_CARGO"
            echo "[SUCCESS] Made indexer independent with empty workspace"
        else
            echo "[INFO] indexer already has workspace configuration"
        fi
    else
        echo "[ERROR] indexer/Cargo.toml not found!"
        exit 1
    fi
fi

echo ""
echo "🚀 Testing the fix..."
if cargo check --quiet; then
    echo "[SUCCESS] Cargo workspace issue fixed!"
    echo ""
    echo "You can now run your setup script again:"
    echo "./setup.sh"
else
    echo "[ERROR] Still having issues. You may need to manually edit the Cargo.toml files."
    echo ""
    echo "Manual fix options:"
    echo "1. Add 'indexer' to workspace.members in parent Cargo.toml"
    echo "2. Add 'indexer' to workspace.exclude in parent Cargo.toml"
    echo "3. Add empty [workspace] to indexer/Cargo.toml"
fi