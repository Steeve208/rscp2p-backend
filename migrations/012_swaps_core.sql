-- Asset swap orders (provider-agnostic routing / execution audit trail)

CREATE TABLE IF NOT EXISTS swap_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_id VARCHAR(64) NOT NULL,
    venue_kind VARCHAR(16) NOT NULL,
    from_asset VARCHAR(32) NOT NULL,
    to_asset VARCHAR(32) NOT NULL,
    from_chain VARCHAR(32),
    to_chain VARCHAR(32),
    from_amount DECIMAL(38, 18) NOT NULL,
    to_amount DECIMAL(38, 18) NOT NULL,
    fee_platform DECIMAL(38, 18) NOT NULL DEFAULT 0,
    fee_provider DECIMAL(38, 18) NOT NULL DEFAULT 0,
    fee_network DECIMAL(38, 18) NOT NULL DEFAULT 0,
    exchange_rate DECIMAL(38, 18),
    slippage_bps INTEGER NOT NULL DEFAULT 50,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    idempotency_key VARCHAR(128) NOT NULL,
    external_order_id VARCHAR(128),
    route_snapshot JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    CONSTRAINT unique_swap_order_idempotency UNIQUE (idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_swap_orders_user ON swap_orders(user_id, status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_swap_orders_external
    ON swap_orders(provider_id, external_order_id)
    WHERE external_order_id IS NOT NULL;

DROP TRIGGER IF EXISTS trg_swap_orders_updated ON swap_orders;
CREATE TRIGGER trg_swap_orders_updated
BEFORE UPDATE ON swap_orders
FOR EACH ROW EXECUTE FUNCTION set_updated_at();
