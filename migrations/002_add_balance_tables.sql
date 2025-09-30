-- Migration: 002_add_balance_tables.sql
-- Add missing columns and indexes for Phase 4

-- Add public_key column to users if not exists (may already exist)
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name='users' AND column_name='public_key') THEN
        ALTER TABLE users ADD COLUMN public_key VARCHAR(88);
    END IF;
END $$;

-- Create assets table if not exists
CREATE TABLE IF NOT EXISTS assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mint_address VARCHAR(44) NOT NULL UNIQUE,
    decimals INTEGER NOT NULL,
    name VARCHAR(255) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    logo_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create balances table if not exists
CREATE TABLE IF NOT EXISTS balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, asset_id)
);

-- Create quotes table if not exists
CREATE TABLE IF NOT EXISTS quotes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    input_mint VARCHAR(44) NOT NULL,
    output_mint VARCHAR(44) NOT NULL,
    in_amount BIGINT NOT NULL,
    out_amount BIGINT NOT NULL,
    price_impact_pct DECIMAL(10, 6),
    quote_data JSONB NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_balances_user_id ON balances(user_id);
CREATE INDEX IF NOT EXISTS idx_balances_asset_id ON balances(asset_id);
CREATE INDEX IF NOT EXISTS idx_balances_user_asset ON balances(user_id, asset_id);
CREATE INDEX IF NOT EXISTS idx_quotes_user_id ON quotes(user_id);
CREATE INDEX IF NOT EXISTS idx_quotes_expires_at ON quotes(expires_at);
CREATE INDEX IF NOT EXISTS idx_quotes_used ON quotes(used);
CREATE INDEX IF NOT EXISTS idx_assets_mint_address ON assets(mint_address);
CREATE INDEX IF NOT EXISTS idx_assets_symbol ON assets(symbol);
CREATE INDEX IF NOT EXISTS idx_users_public_key ON users(public_key);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Insert default assets (SOL, USDC, USDT)
INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at)
VALUES 
    (gen_random_uuid(), 'So11111111111111111111111111111111111111112', 9, 'Solana', 'SOL', 
     'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png', 
     NOW(), NOW()),
    (gen_random_uuid(), 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 6, 'USD Coin', 'USDC', 
     'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png', 
     NOW(), NOW()),
    (gen_random_uuid(), 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 6, 'Tether USD', 'USDT', 
     'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.png', 
     NOW(), NOW())
ON CONFLICT (mint_address) DO NOTHING;
