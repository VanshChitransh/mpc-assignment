-- migrations/002_performance_indexes.sql
-- Performance optimization indexes for Step 1.2

-- ==========================================
-- BALANCES TABLE INDEXES
-- ==========================================

-- Composite index for user-asset lookups (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_balances_user_asset 
    ON balances(user_id, asset_id);

-- Index for finding non-zero balances by user
CREATE INDEX IF NOT EXISTS idx_balances_user_id_amount 
    ON balances(user_id, amount) 
    WHERE amount > 0;

-- ==========================================
-- QUOTES TABLE INDEXES  
-- ==========================================

-- Composite index for user quotes with expiration
CREATE INDEX IF NOT EXISTS idx_quotes_user_expires 
    ON quotes(user_id, expires_at);

-- Partial index for active (non-used) quotes that haven't expired
CREATE INDEX IF NOT EXISTS idx_quotes_expires_at_active 
    ON quotes(expires_at) 
    WHERE used = false;

-- Index for recent quotes queries
CREATE INDEX IF NOT EXISTS idx_quotes_created_at 
    ON quotes(created_at DESC);

-- Index for finding quotes by input/output mints
CREATE INDEX IF NOT EXISTS idx_quotes_mints 
    ON quotes(input_mint, output_mint);

-- ==========================================
-- USERS TABLE INDEXES
-- ==========================================

-- Index for user analytics and recent signups
CREATE INDEX IF NOT EXISTS idx_users_created_at 
    ON users(created_at DESC);

-- Index for case-insensitive email lookups
CREATE INDEX IF NOT EXISTS idx_users_email_lower 
    ON users(LOWER(email));

-- Index on public_key for wallet lookups
CREATE INDEX IF NOT EXISTS idx_users_public_key 
    ON users(public_key) 
    WHERE public_key IS NOT NULL;

-- ==========================================
-- ASSETS TABLE INDEXES
-- ==========================================

-- Index for symbol lookups
CREATE INDEX IF NOT EXISTS idx_assets_symbol 
    ON assets(symbol);

-- ==========================================
-- ANALYZE TABLES FOR QUERY PLANNER
-- ==========================================
ANALYZE users;
ANALYZE assets;
ANALYZE balances;
ANALYZE quotes;
