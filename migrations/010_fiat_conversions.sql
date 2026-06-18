-- Fiat on-ramp orders (Ramp / Transak) linked to invoices and payments.

CREATE TABLE IF NOT EXISTS fiat_conversion_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invoice_id UUID REFERENCES payment_invoices(id) ON DELETE SET NULL,
    payment_id UUID REFERENCES payments(id) ON DELETE SET NULL,
    provider VARCHAR(32) NOT NULL,
    external_order_id VARCHAR(128),
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    fiat_currency VARCHAR(8) NOT NULL,
    fiat_amount DECIMAL(38, 18) NOT NULL,
    crypto_asset VARCHAR(32) NOT NULL,
    crypto_chain VARCHAR(32) NOT NULL,
    crypto_amount DECIMAL(38, 18) NOT NULL,
    exchange_rate DECIMAL(38, 18),
    checkout_url TEXT,
    wallet_address VARCHAR(128),
    idempotency_key VARCHAR(128) NOT NULL,
    provider_metadata JSONB NOT NULL DEFAULT '{}',
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_fiat_order_idempotency UNIQUE (idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_fiat_orders_user ON fiat_conversion_orders(user_id, status);
CREATE INDEX IF NOT EXISTS idx_fiat_orders_invoice ON fiat_conversion_orders(invoice_id)
    WHERE invoice_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_fiat_orders_external
    ON fiat_conversion_orders(provider, external_order_id)
    WHERE external_order_id IS NOT NULL;

DROP TRIGGER IF EXISTS trg_fiat_orders_updated ON fiat_conversion_orders;
CREATE TRIGGER trg_fiat_orders_updated
BEFORE UPDATE ON fiat_conversion_orders
FOR EACH ROW EXECUTE FUNCTION set_updated_at();
