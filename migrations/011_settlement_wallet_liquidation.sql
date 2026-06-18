-- Settlement wallet liquidation (clearing → merchant transfer journal link)

ALTER TABLE settlements
    ADD COLUMN IF NOT EXISTS wallet_journal_id UUID,
    ADD COLUMN IF NOT EXISTS destination_wallet_id UUID REFERENCES wallets(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_settlements_wallet_journal
    ON settlements(wallet_journal_id)
    WHERE wallet_journal_id IS NOT NULL;

-- System user for settlement clearing wallet (funds held until merchant settlement)
INSERT INTO users (id, email, password_hash)
VALUES (
    '00000000-0000-4000-8000-000000000001',
    'settlement-clearing@system.rsc.internal',
    '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'
)
ON CONFLICT (email) DO NOTHING;
