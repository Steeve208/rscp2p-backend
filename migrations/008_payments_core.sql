-- RSC Gateway - Payments Core (RSC Pay foundation)
-- QR payments, merchants, invoices, instant payments, settlements.

-- 1. Merchants (comercios)
CREATE TABLE IF NOT EXISTS merchants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    wallet_id UUID REFERENCES wallets(id) ON DELETE SET NULL,
    display_name VARCHAR(120) NOT NULL,
    legal_name VARCHAR(200),
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    settlement_asset VARCHAR(32) NOT NULL DEFAULT 'RSC',
    settlement_chain VARCHAR(32) NOT NULL DEFAULT 'rsc-mainnet',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_merchants_owner ON merchants(owner_user_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_merchants_owner_display
ON merchants(owner_user_id, display_name);

-- 2. Invoices (facturas / QR payment requests)
CREATE TABLE IF NOT EXISTS payment_invoices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id UUID NOT NULL REFERENCES merchants(id) ON DELETE CASCADE,
    reference_code VARCHAR(32) NOT NULL,
    amount DECIMAL(38,18) NOT NULL,
    asset VARCHAR(32) NOT NULL,
    chain VARCHAR(32) NOT NULL,
    description TEXT,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    expires_at TIMESTAMPTZ,
    idempotency_key VARCHAR(128),
    paid_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_invoice_reference UNIQUE (reference_code)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_invoice_merchant_idempotency
ON payment_invoices(merchant_id, idempotency_key)
WHERE idempotency_key IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_invoices_merchant_status ON payment_invoices(merchant_id, status);

-- 3. Payments (motor de pagos — instant / QR completion)
CREATE TABLE IF NOT EXISTS payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id UUID NOT NULL REFERENCES payment_invoices(id) ON DELETE RESTRICT,
    payer_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    amount DECIMAL(38,18) NOT NULL,
    fee DECIMAL(38,18) NOT NULL DEFAULT 0,
    method VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    idempotency_key VARCHAR(128) NOT NULL,
    wallet_transaction_id UUID,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_payment_idempotency UNIQUE (idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_payments_invoice ON payments(invoice_id);
CREATE INDEX IF NOT EXISTS idx_payments_payer ON payments(payer_user_id);

-- 4. Settlements (liquidaciones a comercios)
CREATE TABLE IF NOT EXISTS settlements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id UUID NOT NULL REFERENCES merchants(id) ON DELETE CASCADE,
    amount DECIMAL(38,18) NOT NULL,
    asset VARCHAR(32) NOT NULL,
    chain VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    period_start TIMESTAMPTZ,
    period_end TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_settlements_merchant ON settlements(merchant_id, status);

-- 5. Settlement line items (pagos incluidos en una liquidación)
CREATE TABLE IF NOT EXISTS settlement_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL REFERENCES settlements(id) ON DELETE CASCADE,
    payment_id UUID NOT NULL REFERENCES payments(id) ON DELETE RESTRICT,
    amount DECIMAL(38,18) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_settlement_payment UNIQUE (settlement_id, payment_id)
);

CREATE INDEX IF NOT EXISTS idx_settlement_items_payment ON settlement_items(payment_id);

DROP TRIGGER IF EXISTS trg_merchants_updated ON merchants;
CREATE TRIGGER trg_merchants_updated
BEFORE UPDATE ON merchants
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_invoices_updated ON payment_invoices;
CREATE TRIGGER trg_invoices_updated
BEFORE UPDATE ON payment_invoices
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_payments_updated ON payments;
CREATE TRIGGER trg_payments_updated
BEFORE UPDATE ON payments
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_settlements_updated ON settlements;
CREATE TRIGGER trg_settlements_updated
BEFORE UPDATE ON settlements
FOR EACH ROW EXECUTE FUNCTION set_updated_at();
