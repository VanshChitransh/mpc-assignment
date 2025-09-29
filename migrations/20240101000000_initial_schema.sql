-- Initial schema for MPC Solana Wallet

-- Drop tables if they exist (for clean setup)
DROP TABLE IF EXISTS quotes CASCADE;
DROP TABLE IF EXISTS balances CASCADE;
DROP TABLE IF EXISTS assets CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    public_key VARCHAR(44),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_public_key ON users(public_key) WHERE public_key IS NOT NULL;

-- Assets table (for tokens)
CREATE TABLE assets (
    id UUID PRIMARY KEY,
    mint_address VARCHAR(44) UNIQUE NOT NULL,
    decimals INTEGER NOT NULL CHECK (decimals >= 0 AND decimals <= 18),
    name VARCHAR(255) NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    logo_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_assets_mint_address ON assets(mint_address);
CREATE INDEX idx_assets_symbol ON assets(symbol);

-- Balances table
CREATE TABLE balances (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL DEFAULT 0 CHECK (amount >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_asset UNIQUE(user_id, asset_id)
);

CREATE INDEX idx_balances_user_asset ON balances(user_id, asset_id);
CREATE INDEX idx_balances_user_id ON balances(user_id);
CREATE INDEX idx_balances_amount ON balances(amount) WHERE amount > 0;

-- Quotes table (for Jupiter swaps)
CREATE TABLE quotes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    input_mint VARCHAR(44) NOT NULL,
    output_mint VARCHAR(44) NOT NULL,
    in_amount BIGINT NOT NULL CHECK (in_amount > 0),
    out_amount BIGINT NOT NULL CHECK (out_amount > 0),
    quote_data JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_quotes_user_id ON quotes(user_id);
CREATE INDEX idx_quotes_user_expires ON quotes(user_id, expires_at);
CREATE INDEX idx_quotes_expires_at ON quotes(expires_at) WHERE used = FALSE;
CREATE INDEX idx_quotes_used ON quotes(used);

-- Insert default assets
INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url) VALUES
    ('550e8400-e29b-41d4-a716-446655440000', 'So11111111111111111111111111111111111111112', 9, 'Solana', 'SOL', 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png'),
    ('550e8400-e29b-41d4-a716-446655440001', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 6, 'USD Coin', 'USDC', 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png'),
    ('550e8400-e29b-41d4-a716-446655440002', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 6, 'Tether USD', 'USDT', 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.png')
ON CONFLICT (mint_address) DO NOTHING;
