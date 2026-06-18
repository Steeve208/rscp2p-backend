-- RSC Gateway - Wallets Financial Hardening
-- Enforce idempotency and common lookup guarantees before money-moving flows exist.

CREATE UNIQUE INDEX IF NOT EXISTS idx_ledger_idempotency_unique
ON ledger_entries(idempotency_key)
WHERE idempotency_key IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wallet_addresses_wallet_asset_chain
ON wallet_addresses(wallet_id, asset, chain);

CREATE INDEX IF NOT EXISTS idx_tx_wallet_asset_chain
ON wallet_transactions(wallet_id, asset, chain);
