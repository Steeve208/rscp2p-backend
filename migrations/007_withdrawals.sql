-- Withdrawal support: idempotency on transactions + faster status lookups

ALTER TABLE wallet_transactions
    ADD COLUMN IF NOT EXISTS idempotency_key VARCHAR(128);

CREATE UNIQUE INDEX IF NOT EXISTS idx_tx_wallet_idempotency
ON wallet_transactions(wallet_id, idempotency_key)
WHERE idempotency_key IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_tx_withdrawal_pending
ON wallet_transactions(wallet_id, status)
WHERE type = 'withdrawal' AND status IN ('pending', 'confirming');
