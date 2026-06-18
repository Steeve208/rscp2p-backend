-- RSC Gateway - Wallets Core (Financial Engine)
-- This module is ONLY wallet integration + accounting. No full node logic.

-- 1. Wallets (a user can have multiple wallets in the future)
CREATE TABLE IF NOT EXISTS wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label VARCHAR(120),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wallets_user ON wallets(user_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_wallets_user_default ON wallets(user_id) WHERE is_default = TRUE;

-- 2. Balances per wallet + asset + chain (materialized for performance)
CREATE TABLE IF NOT EXISTS wallet_balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    asset VARCHAR(32) NOT NULL,           -- e.g. "BTC", "ETH", "RSC", "USDT"
    chain VARCHAR(32) NOT NULL,           -- e.g. "bitcoin", "ethereum", "rsc-mainnet"
    available DECIMAL(38,18) NOT NULL DEFAULT 0,   -- spendable
    total DECIMAL(38,18) NOT NULL DEFAULT 0,       -- including locked/pending
    locked DECIMAL(38,18) NOT NULL DEFAULT 0,      -- withdrawals, holds, etc.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_wallet_asset_chain UNIQUE (wallet_id, asset, chain)
);

CREATE INDEX IF NOT EXISTS idx_balances_wallet ON wallet_balances(wallet_id);
CREATE INDEX IF NOT EXISTS idx_balances_asset_chain ON wallet_balances(asset, chain);

-- 3. Deposit / receive addresses
CREATE TABLE IF NOT EXISTS wallet_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    asset VARCHAR(32) NOT NULL,
    chain VARCHAR(32) NOT NULL,
    address VARCHAR(128) NOT NULL,
    derivation_path TEXT,
    is_used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_address_per_chain UNIQUE (chain, address)
);

CREATE INDEX IF NOT EXISTS idx_addresses_wallet ON wallet_addresses(wallet_id);
CREATE INDEX IF NOT EXISTS idx_addresses_asset_chain ON wallet_addresses(asset, chain);

-- 4. Double-entry ledger (the source of truth for all money movement)
CREATE TABLE IF NOT EXISTS ledger_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    journal_id UUID NOT NULL,                    -- groups debit + credit of one operation
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE RESTRICT,
    asset VARCHAR(32) NOT NULL,
    chain VARCHAR(32) NOT NULL,
    amount DECIMAL(38,18) NOT NULL,              -- positive = credit (increase), negative = debit
    entry_type VARCHAR(32) NOT NULL,             -- deposit, withdrawal, internal_transfer, fee, adjustment, etc.
    related_wallet_id UUID REFERENCES wallets(id),
    transaction_id UUID,                         -- link to wallet_transactions if exists
    idempotency_key VARCHAR(128),
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ledger_wallet ON ledger_entries(wallet_id);
CREATE INDEX IF NOT EXISTS idx_ledger_journal ON ledger_entries(journal_id);
CREATE INDEX IF NOT EXISTS idx_ledger_idempotency ON ledger_entries(idempotency_key) WHERE idempotency_key IS NOT NULL;

-- 5. Higher-level transaction view (for UI/history)
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    type VARCHAR(32) NOT NULL,                   -- deposit, withdrawal, transfer_in, transfer_out, fee
    asset VARCHAR(32) NOT NULL,
    chain VARCHAR(32) NOT NULL,
    amount DECIMAL(38,18) NOT NULL,
    fee DECIMAL(38,18) NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'pending', -- pending, confirming, confirmed, failed, cancelled
    tx_hash VARCHAR(128),
    from_address VARCHAR(128),
    to_address VARCHAR(128),
    block_height BIGINT,
    confirmations INT DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tx_wallet ON wallet_transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_tx_status ON wallet_transactions(status);
CREATE INDEX IF NOT EXISTS idx_tx_hash ON wallet_transactions(tx_hash);

-- Trigger to maintain updated_at on wallets
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_wallets_updated ON wallets;
CREATE TRIGGER trg_wallets_updated
BEFORE UPDATE ON wallets
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Note: Balances are updated via application logic (or triggers in more advanced setups)
-- We intentionally do NOT put heavy triggers here to keep the accounting explicit in code.
