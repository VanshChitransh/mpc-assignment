-- migrations/001_initial_schema.sql
-- Initial schema for Solana wallet backend

-- Users table (updated to include public_key and proper timestamp handling)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR UNIQUE NOT NULL,
    password_hash VARCHAR NOT NULL,
    public_key VARCHAR, -- Solana public key from MPC
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Assets table (supported tokens)
CREATE TABLE IF NOT EXISTS assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mint_address VARCHAR UNIQUE NOT NULL, -- Solana mint address
    decimals INTEGER NOT NULL,
    name VARCHAR NOT NULL,
    symbol VARCHAR NOT NULL,
    logo_url VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Balances table (user token balances)
CREATE TABLE IF NOT EXISTS balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    amount BIGINT NOT NULL DEFAULT 0, -- stored in smallest units (lamports for SOL)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    UNIQUE(user_id, asset_id)
);

-- Quotes table (Jupiter swap quotes)
CREATE TABLE IF NOT EXISTS quotes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    input_mint VARCHAR NOT NULL,
    output_mint VARCHAR NOT NULL,
    in_amount BIGINT NOT NULL,
    out_amount BIGINT NOT NULL,
    quote_data JSONB NOT NULL, -- Full Jupiter quote response
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- CREATE TRIGGER update_quotes_updated_at BEFORE UPDATE ON quotes FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Keyshares table (for MPC nodes)
CREATE TABLE IF NOT EXISTS keyshares (
    user_id UUID NOT NULL,
    public_key VARCHAR NOT NULL,
    private_key VARCHAR NOT NULL, -- encrypted key share
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(user_id)
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_public_key ON users(public_key);
CREATE INDEX IF NOT EXISTS idx_balances_user_id ON balances(user_id);
CREATE INDEX IF NOT EXISTS idx_balances_asset_id ON balances(asset_id);
CREATE INDEX IF NOT EXISTS idx_balances_user_asset ON balances(user_id, asset_id);
CREATE INDEX IF NOT EXISTS idx_quotes_user_id ON quotes(user_id);
CREATE INDEX IF NOT EXISTS idx_quotes_expires_at ON quotes(expires_at);
CREATE INDEX IF NOT EXISTS idx_quotes_used ON quotes(used);
CREATE INDEX IF NOT EXISTS idx_assets_mint_address ON assets(mint_address);
CREATE INDEX IF NOT EXISTS idx_keyshares_user_id ON keyshares(user_id);

-- Insert default assets (SOL and USDC)
INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url) VALUES
    (gen_random_uuid(), 'So11111111111111111111111111111111111111112', 9, 'Solana', 'SOL', NULL),
    (gen_random_uuid(), 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 6, 'USD Coin', 'USDC', NULL),
    (gen_random_uuid(), 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 6, 'Tether USD', 'USDT', NULL)
ON CONFLICT (id) DO NOTHING;

-- Add trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_assets_updated_at BEFORE UPDATE ON assets  
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_balances_updated_at BEFORE UPDATE ON balances
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();