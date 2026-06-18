-- RSC Bank: Striga banking, cards, KYC, crypto orders, webhook audit

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS striga_user_id VARCHAR(128) UNIQUE;

CREATE INDEX IF NOT EXISTS idx_users_striga_user_id ON users (striga_user_id)
    WHERE striga_user_id IS NOT NULL;

-- KYC verification records (RSC-branded status shown to users)
CREATE TABLE IF NOT EXISTS kyc_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    tier SMALLINT NOT NULL DEFAULT 1,
    striga_user_id VARCHAR(128),
    verification_token TEXT,
    rejection_reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id)
);

CREATE INDEX IF NOT EXISTS idx_kyc_records_status ON kyc_records (status);

-- Issued payment cards (Visa via Striga — never exposed as Striga to end users)
CREATE TABLE IF NOT EXISTS cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    striga_card_id VARCHAR(128) NOT NULL UNIQUE,
    card_type VARCHAR(16) NOT NULL,
    card_status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    last_four VARCHAR(4),
    expiry_month SMALLINT,
    expiry_year SMALLINT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cards_user_id ON cards (user_id);
CREATE INDEX IF NOT EXISTS idx_cards_status ON cards (card_status);

-- Card transaction history (synced from provider)
CREATE TABLE IF NOT EXISTS card_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    card_id UUID NOT NULL REFERENCES cards (id) ON DELETE CASCADE,
    external_id VARCHAR(128) NOT NULL,
    amount NUMERIC(20, 8) NOT NULL,
    currency VARCHAR(8) NOT NULL,
    direction VARCHAR(8) NOT NULL,
    merchant_name TEXT,
    status VARCHAR(32) NOT NULL,
    transacted_at TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (card_id, external_id)
);

CREATE INDEX IF NOT EXISTS idx_card_transactions_card_id ON card_transactions (card_id, transacted_at DESC);

-- Linked bank accounts (IBAN / SEPA)
CREATE TABLE IF NOT EXISTS bank_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    striga_account_id VARCHAR(128),
    iban VARCHAR(34),
    bic VARCHAR(11),
    currency VARCHAR(8) NOT NULL DEFAULT 'EUR',
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bank_accounts_user_id ON bank_accounts (user_id);

-- Crypto buy/sell orders (Transak — white-label)
CREATE TABLE IF NOT EXISTS crypto_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider VARCHAR(16) NOT NULL DEFAULT 'transak',
    external_order_id VARCHAR(128),
    order_type VARCHAR(16) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    fiat_currency VARCHAR(8),
    fiat_amount NUMERIC(20, 8),
    crypto_asset VARCHAR(16),
    crypto_amount NUMERIC(20, 8),
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_crypto_orders_user_id ON crypto_orders (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crypto_orders_external ON crypto_orders (external_order_id)
    WHERE external_order_id IS NOT NULL;

-- Webhook audit log (all providers)
CREATE TABLE IF NOT EXISTS webhook_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider VARCHAR(32) NOT NULL,
    event_type VARCHAR(64) NOT NULL,
    external_id VARCHAR(128),
    payload JSONB NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT FALSE,
    error_message TEXT,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_webhook_logs_provider ON webhook_logs (provider, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_unprocessed ON webhook_logs (processed)
    WHERE processed = FALSE;

-- Provider health snapshots (admin dashboard)
CREATE TABLE IF NOT EXISTS provider_status (
    provider VARCHAR(32) PRIMARY KEY,
    status VARCHAR(16) NOT NULL DEFAULT 'unknown',
    last_sync_at TIMESTAMPTZ,
    last_error TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO provider_status (provider, status) VALUES
    ('striga', 'unknown'),
    ('transak', 'unknown')
ON CONFLICT (provider) DO NOTHING;
