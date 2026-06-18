-- Link completed payments to wallet journal (internal transfer)

ALTER TABLE payments
    ADD COLUMN IF NOT EXISTS wallet_journal_id UUID;

CREATE INDEX IF NOT EXISTS idx_payments_wallet_journal
ON payments(wallet_journal_id)
WHERE wallet_journal_id IS NOT NULL;
