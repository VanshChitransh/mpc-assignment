-- Migration: 003_wallet_state_management.sql
-- Add wallet_keys and signing_sessions tables

-- Create wallet_keys table if not exists
CREATE TABLE IF NOT EXISTS wallet_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_key VARCHAR(88) NOT NULL,
    threshold INTEGER NOT NULL DEFAULT 2,
    total_parties INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create signing_sessions table if not exists
CREATE TABLE IF NOT EXISTS signing_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message_hash VARCHAR(128) NOT NULL,
    nonce_commitment TEXT,
    signing_package TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW() + INTERVAL '5 minutes'
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_signing_sessions_user_id ON signing_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_status ON signing_sessions(status);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_expires_at ON signing_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_message_hash ON signing_sessions(message_hash);
CREATE INDEX IF NOT EXISTS idx_wallet_keys_user_id ON wallet_keys(user_id);
