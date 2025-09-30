#!/bin/bash

set -e

echo "Fixing database permissions..."

# Connect as superuser and fix permissions
psql -d solana_wallet_temp -c "GRANT ALL ON SCHEMA public TO postgres;"
psql -d solana_wallet_temp -c "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO postgres;"
psql -d solana_wallet_temp -c "GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO postgres;"
psql -d solana_wallet_temp -c "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO postgres;"
psql -d solana_wallet_temp -c "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO postgres;"

echo "Permissions fixed!"
