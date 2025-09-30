-- Migration 003: Wallet State Management for Phase 3

-- Drop existing tables if they exist (for clean migration)
DROP TABLE IF EXISTS signing_sessions CASCADE;
DROP TABLE IF EXISTS wallet_keys CASCADE;

-- Create wallet_keys table
CREATE TABLE IF NOT EXISTS wallet_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_key VARCHAR(88) NOT NULL,
    threshold INTEGER NOT NULL DEFAULT 2,
    total_parties INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create signing_sessions table (using VARCHAR instead of enum for compatibility)
CREATE TABLE IF NOT EXISTS signing_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message_hash VARCHAR(128) NOT NULL,
    nonce_commitment TEXT,
    signing_package TEXT,
    signature_shares TEXT[] DEFAULT '{}',
    final_signature VARCHAR,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW() + INTERVAL '30 minutes',
    CONSTRAINT check_status_values CHECK (status IN ('pending', 'phase1', 'phase2', 'completed', 'failed', 'expired'))
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_signing_sessions_user_id ON signing_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_status ON signing_sessions(status);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_expires_at ON signing_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_signing_sessions_message_hash ON signing_sessions(message_hash);

-- Create index for wallet_keys
CREATE INDEX IF NOT EXISTS idx_wallet_keys_public_key ON wallet_keys(public_key);

COMMENT ON TABLE wallet_keys IS 'Stores MPC-generated public keys for users';
COMMENT ON TABLE signing_sessions IS 'Tracks MPC signing sessions with expiration';
