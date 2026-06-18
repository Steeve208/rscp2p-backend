//! Wallet Repository — Database access for wallets, balances, addresses and ledger.
//!
//! All monetary operations should eventually go through the ledger for double-entry integrity.

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::wallets::error::{WalletError, WalletResult};
use crate::internal::wallets::models::{
    Asset, Chain, DepositTarget, InternalTransferRequest, InternalTransferResponse,
    LedgerEntry, LedgerEntryType, RecordDepositRequest, RecordDepositResponse,
    RequestWithdrawalRequest, TransactionStatus, TransactionType, Wallet, WalletAddress,
    WalletBalance, WalletTransaction,
};

#[derive(Clone)]
pub struct WalletRepository {
    pool: PgPool,
}

impl WalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_wallets_by_user(&self, user_id: Uuid) -> WalletResult<Vec<Wallet>> {
        let rows = sqlx::query_as::<_, WalletRow>(
            r#"
            SELECT id, user_id, label, is_default, created_at, updated_at
            FROM wallets
            WHERE user_id = $1
            ORDER BY is_default DESC, created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn ensure_default_wallet(&self, user_id: Uuid) -> WalletResult<Wallet> {
        let row = sqlx::query_as::<_, WalletRow>(
            r#"
            INSERT INTO wallets (user_id, label, is_default)
            VALUES ($1, 'Default', TRUE)
            ON CONFLICT (user_id) WHERE is_default = TRUE
            DO UPDATE SET updated_at = wallets.updated_at
            RETURNING id, user_id, label, is_default, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn list_balances_by_user(&self, user_id: Uuid) -> WalletResult<Vec<WalletBalance>> {
        let rows = sqlx::query_as::<_, WalletBalanceRow>(
            r#"
            SELECT b.wallet_id, b.asset, b.chain, b.available, b.total, b.locked, b.updated_at
            FROM wallet_balances b
            INNER JOIN wallets w ON w.id = b.wallet_id
            WHERE w.user_id = $1
            ORDER BY b.asset ASC, b.chain ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn find_deposit_target_by_address(
        &self,
        address: &str,
    ) -> WalletResult<Option<DepositTarget>> {
        let normalized = address.trim().to_lowercase();
        let row = sqlx::query_as::<_, DepositTargetRow>(
            r#"
            SELECT wallet_id, asset, chain
            FROM wallet_addresses
            WHERE LOWER(address) = $1
            LIMIT 1
            "#,
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_addresses_by_user(&self, user_id: Uuid) -> WalletResult<Vec<WalletAddress>> {
        let rows = sqlx::query_as::<_, WalletAddressRow>(
            r#"
            SELECT a.id, a.wallet_id, a.asset, a.chain, a.address, a.derivation_path,
                   a.is_used, a.created_at
            FROM wallet_addresses a
            INNER JOIN wallets w ON w.id = a.wallet_id
            WHERE w.user_id = $1
            ORDER BY a.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_or_create_deposit_address(
        &self,
        user_id: Uuid,
        asset: &str,
        chain: &str,
        address: &str,
    ) -> WalletResult<(Wallet, WalletAddress)> {
        let mut tx = self.pool.begin().await?;

        let wallet = sqlx::query_as::<_, WalletRow>(
            r#"
            INSERT INTO wallets (user_id, label, is_default)
            VALUES ($1, 'Default', TRUE)
            ON CONFLICT (user_id) WHERE is_default = TRUE
            DO UPDATE SET updated_at = wallets.updated_at
            RETURNING id, user_id, label, is_default, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO wallet_balances (wallet_id, asset, chain)
            VALUES ($1, $2, $3)
            ON CONFLICT (wallet_id, asset, chain)
            DO UPDATE SET updated_at = wallet_balances.updated_at
            "#,
        )
        .bind(wallet.id)
        .bind(asset)
        .bind(chain)
        .execute(&mut *tx)
        .await?;

        let address = sqlx::query_as::<_, WalletAddressRow>(
            r#"
            INSERT INTO wallet_addresses (wallet_id, asset, chain, address)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (wallet_id, asset, chain)
            DO UPDATE SET address = wallet_addresses.address
            RETURNING id, wallet_id, asset, chain, address, derivation_path, is_used, created_at
            "#,
        )
        .bind(wallet.id)
        .bind(asset)
        .bind(chain)
        .bind(address)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok((wallet.into(), address.into()))
    }

    pub async fn list_transactions_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> WalletResult<Vec<WalletTransaction>> {
        let rows = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            SELECT t.id, t.wallet_id, t.type AS tx_type, t.asset, t.chain, t.amount, t.fee,
                   t.status, t.tx_hash, t.from_address, t.to_address, t.confirmations,
                   t.created_at, t.updated_at
            FROM wallet_transactions t
            INNER JOIN wallets w ON w.id = t.wallet_id
            WHERE w.user_id = $1
            ORDER BY t.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn list_ledger_entries_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> WalletResult<Vec<LedgerEntry>> {
        let rows = sqlx::query_as::<_, LedgerEntryRow>(
            r#"
            SELECT l.id, l.journal_id, l.wallet_id, l.asset, l.chain, l.amount,
                   l.entry_type, l.related_wallet_id, l.transaction_id,
                   l.idempotency_key, l.metadata, l.created_at
            FROM ledger_entries l
            INNER JOIN wallets w ON w.id = l.wallet_id
            WHERE w.user_id = $1
            ORDER BY l.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn record_confirmed_deposit(
        &self,
        req: &RecordDepositRequest,
    ) -> WalletResult<RecordDepositResponse> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            SELECT id
            FROM wallets
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(req.wallet_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(WalletError::WalletNotFound)?;

        if let Some(existing) = self
            .find_deposit_by_idempotency_key(&mut tx, &req.idempotency_key)
            .await?
        {
            tx.commit().await?;
            return Ok(existing);
        }

        let metadata = req
            .metadata
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));

        let transaction = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            INSERT INTO wallet_transactions (
                wallet_id, type, asset, chain, amount, fee, status, tx_hash,
                from_address, to_address, confirmations, metadata
            )
            VALUES ($1, 'deposit', $2, $3, $4, 0, 'confirmed', $5, $6, $7, $8, $9)
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(req.wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .bind(&req.tx_hash)
        .bind(&req.from_address)
        .bind(&req.to_address)
        .bind(req.confirmations)
        .bind(&metadata)
        .fetch_one(&mut *tx)
        .await?;

        let ledger_entry = sqlx::query_as::<_, LedgerEntryRow>(
            r#"
            INSERT INTO ledger_entries (
                journal_id, wallet_id, asset, chain, amount, entry_type,
                transaction_id, idempotency_key, metadata
            )
            VALUES (gen_random_uuid(), $1, $2, $3, $4, 'deposit', $5, $6, $7)
            RETURNING id, journal_id, wallet_id, asset, chain, amount, entry_type,
                      related_wallet_id, transaction_id, idempotency_key, metadata, created_at
            "#,
        )
        .bind(req.wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .bind(transaction.id)
        .bind(&req.idempotency_key)
        .bind(&metadata)
        .fetch_one(&mut *tx)
        .await?;

        let balance = sqlx::query_as::<_, WalletBalanceRow>(
            r#"
            INSERT INTO wallet_balances (wallet_id, asset, chain, available, total, locked)
            VALUES ($1, $2, $3, $4, $4, 0)
            ON CONFLICT (wallet_id, asset, chain)
            DO UPDATE SET
                available = wallet_balances.available + EXCLUDED.available,
                total = wallet_balances.total + EXCLUDED.total,
                updated_at = NOW()
            RETURNING wallet_id, asset, chain, available, total, locked, updated_at
            "#,
        )
        .bind(req.wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(RecordDepositResponse {
            transaction: transaction.try_into()?,
            balance: balance.into(),
            ledger_entry: ledger_entry.try_into()?,
            idempotent_replay: false,
        })
    }

    pub async fn wallet_belongs_to_user(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
    ) -> WalletResult<bool> {
        let found: Option<Uuid> =
            sqlx::query_scalar(r#"SELECT id FROM wallets WHERE id = $1 AND user_id = $2"#)
                .bind(wallet_id)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(found.is_some())
    }

    pub async fn get_default_wallet_id(&self, user_id: Uuid) -> WalletResult<Uuid> {
        let wallet = self.ensure_default_wallet(user_id).await?;
        Ok(wallet.id)
    }

    /// Global wallet that holds customer payments until merchant settlement.
    pub async fn ensure_clearing_wallet(&self) -> WalletResult<Uuid> {
        const CLEARING_EMAIL: &str = "settlement-clearing@system.rsc.internal";

        let user_id: Uuid = sqlx::query_scalar(
            r#"SELECT id FROM users WHERE email = $1"#,
        )
        .bind(CLEARING_EMAIL)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            WalletError::Internal(anyhow::anyhow!(
                "settlement clearing user missing; run migrations"
            ))
        })?;

        let wallet = self.ensure_default_wallet(user_id).await?;
        sqlx::query(
            r#"
            UPDATE wallets
            SET label = 'Settlement Clearing', updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(wallet.id)
        .execute(&self.pool)
        .await?;

        Ok(wallet.id)
    }

    pub async fn get_balance_for_wallet(
        &self,
        wallet_id: Uuid,
        asset: &str,
        chain: &str,
    ) -> WalletResult<Option<WalletBalance>> {
        let row = sqlx::query_as::<_, WalletBalanceRow>(
            r#"
            SELECT wallet_id, asset, chain, available, total, locked, updated_at
            FROM wallet_balances
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3
            "#,
        )
        .bind(wallet_id)
        .bind(asset)
        .bind(chain)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn get_transaction_for_user(
        &self,
        user_id: Uuid,
        transaction_id: Uuid,
    ) -> WalletResult<Option<WalletTransaction>> {
        let row = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            SELECT t.id, t.wallet_id, t.type AS tx_type, t.asset, t.chain, t.amount, t.fee,
                   t.status, t.tx_hash, t.from_address, t.to_address, t.confirmations,
                   t.created_at, t.updated_at
            FROM wallet_transactions t
            INNER JOIN wallets w ON w.id = t.wallet_id
            WHERE t.id = $1 AND w.user_id = $2
            "#,
        )
        .bind(transaction_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn request_withdrawal(
        &self,
        user_id: Uuid,
        wallet_id: Uuid,
        req: &RequestWithdrawalRequest,
        asset: &str,
        chain: &str,
        fee: Decimal,
    ) -> WalletResult<(WalletTransaction, bool)> {
        let mut tx = self.pool.begin().await?;

        let owner: Option<Uuid> =
            sqlx::query_scalar(r#"SELECT user_id FROM wallets WHERE id = $1 FOR UPDATE"#)
                .bind(wallet_id)
                .fetch_optional(&mut *tx)
                .await?;

        if owner != Some(user_id) {
            return Err(WalletError::Forbidden);
        }

        if let Some(existing) = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            SELECT id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                   status, tx_hash, from_address, to_address, confirmations,
                   created_at, updated_at
            FROM wallet_transactions
            WHERE wallet_id = $1 AND idempotency_key = $2
            "#,
        )
        .bind(wallet_id)
        .bind(&req.idempotency_key)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok((existing.try_into()?, true));
        }

        sqlx::query(
            r#"
            INSERT INTO wallet_balances (wallet_id, asset, chain)
            VALUES ($1, $2, $3)
            ON CONFLICT (wallet_id, asset, chain) DO NOTHING
            "#,
        )
        .bind(wallet_id)
        .bind(asset)
        .bind(chain)
        .execute(&mut *tx)
        .await?;

        let total_hold = req.amount + fee;
        let updated = sqlx::query(
            r#"
            UPDATE wallet_balances
            SET
                available = available - $4,
                locked = locked + $4,
                updated_at = NOW()
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3 AND available >= $4
            "#,
        )
        .bind(wallet_id)
        .bind(asset)
        .bind(chain)
        .bind(total_hold)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(WalletError::InsufficientBalance);
        }

        let metadata = serde_json::json!({
            "idempotency_key": req.idempotency_key,
            "requested_by": user_id,
        });

        let transaction = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            INSERT INTO wallet_transactions (
                wallet_id, type, asset, chain, amount, fee, status,
                to_address, idempotency_key, metadata
            )
            VALUES ($1, 'withdrawal', $2, $3, $4, $5, 'pending', $6, $7, $8)
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(wallet_id)
        .bind(asset)
        .bind(chain)
        .bind(req.amount)
        .bind(fee)
        .bind(&req.to_address)
        .bind(&req.idempotency_key)
        .bind(&metadata)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((transaction.try_into()?, false))
    }

    pub async fn cancel_withdrawal(
        &self,
        user_id: Uuid,
        transaction_id: Uuid,
    ) -> WalletResult<WalletTransaction> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, WithdrawalLockRow>(
            r#"
            SELECT t.id, t.wallet_id, t.asset, t.chain, t.amount, t.fee, t.status AS tx_status
            FROM wallet_transactions t
            INNER JOIN wallets w ON w.id = t.wallet_id
            WHERE t.id = $1 AND w.user_id = $2 AND t.type = 'withdrawal'
            FOR UPDATE OF t
            "#,
        )
        .bind(transaction_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(WalletError::WalletNotFound)?;

        if row.tx_status != "pending" {
            return Err(WalletError::Validation(
                "only pending withdrawals can be cancelled".into(),
            ));
        }

        let hold = row.amount + row.fee;
        sqlx::query(
            r#"
            UPDATE wallet_balances
            SET available = available + $4, locked = locked - $4, updated_at = NOW()
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3
            "#,
        )
        .bind(row.wallet_id)
        .bind(&row.asset)
        .bind(&row.chain)
        .bind(hold)
        .execute(&mut *tx)
        .await?;

        let transaction = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            UPDATE wallet_transactions
            SET status = 'cancelled', updated_at = NOW()
            WHERE id = $1
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(transaction_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(transaction.try_into()?)
    }

    pub async fn mark_withdrawal_broadcast(
        &self,
        user_id: Uuid,
        transaction_id: Uuid,
        tx_hash: &str,
    ) -> WalletResult<WalletTransaction> {
        let mut tx = self.pool.begin().await?;

        let status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT t.status
            FROM wallet_transactions t
            INNER JOIN wallets w ON w.id = t.wallet_id
            WHERE t.id = $1 AND w.user_id = $2 AND t.type = 'withdrawal'
            FOR UPDATE OF t
            "#,
        )
        .bind(transaction_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(status) = status else {
            return Err(WalletError::WalletNotFound);
        };
        if status != "pending" {
            return Err(WalletError::Validation(
                "withdrawal must be pending to broadcast".into(),
            ));
        }

        let transaction = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            UPDATE wallet_transactions
            SET status = 'confirming', tx_hash = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(transaction_id)
        .bind(tx_hash)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(transaction.try_into()?)
    }

    pub async fn list_confirming_withdrawals(
        &self,
        limit: i64,
    ) -> WalletResult<Vec<WalletTransaction>> {
        let rows = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            SELECT id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                   status, tx_hash, from_address, to_address, confirmations,
                   created_at, updated_at
            FROM wallet_transactions
            WHERE type = 'withdrawal' AND status = 'confirming' AND tx_hash IS NOT NULL
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn finalize_confirmed_withdrawal(
        &self,
        transaction_id: Uuid,
        tx_hash: &str,
        confirmations: i32,
    ) -> WalletResult<Option<WalletTransaction>> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, WithdrawalLockRow>(
            r#"
            SELECT t.id, t.wallet_id, t.asset, t.chain, t.amount, t.fee, t.status AS tx_status
            FROM wallet_transactions t
            WHERE t.id = $1 AND t.type = 'withdrawal' AND t.tx_hash = $2
            FOR UPDATE OF t
            "#,
        )
        .bind(transaction_id)
        .bind(tx_hash)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(WalletError::WalletNotFound)?;

        if row.tx_status == "confirmed" {
            let existing = sqlx::query_as::<_, WalletTransactionRow>(
                r#"
                SELECT id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                       status, tx_hash, from_address, to_address, confirmations,
                       created_at, updated_at
                FROM wallet_transactions WHERE id = $1
                "#,
            )
            .bind(transaction_id)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(Some(existing.try_into()?));
        }

        if row.tx_status != "confirming" {
            tx.commit().await?;
            return Ok(None);
        }

        let idempotency_key = format!("withdrawal:{transaction_id}");
        if sqlx::query_scalar::<_, i64>(
            r#"SELECT 1 FROM ledger_entries WHERE idempotency_key = $1"#,
        )
        .bind(&idempotency_key)
        .fetch_optional(&mut *tx)
        .await?
        .is_some()
        {
            tx.commit().await?;
            return Ok(None);
        }

        let hold = row.amount + row.fee;
        let debit_amount = -row.amount;

        sqlx::query(
            r#"
            UPDATE wallet_balances
            SET
                locked = locked - $4,
                total = total - $4,
                updated_at = NOW()
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3
            "#,
        )
        .bind(row.wallet_id)
        .bind(&row.asset)
        .bind(&row.chain)
        .bind(hold)
        .execute(&mut *tx)
        .await?;

        let metadata = serde_json::json!({
            "source": "withdrawal_worker",
            "tx_hash": tx_hash,
        });

        let transaction = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            UPDATE wallet_transactions
            SET status = 'confirmed', confirmations = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(transaction_id)
        .bind(confirmations)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO ledger_entries (
                journal_id, wallet_id, asset, chain, amount, entry_type,
                transaction_id, idempotency_key, metadata
            )
            VALUES (gen_random_uuid(), $1, $2, $3, $4, 'withdrawal', $5, $6, $7)
            "#,
        )
        .bind(row.wallet_id)
        .bind(&row.asset)
        .bind(&row.chain)
        .bind(debit_amount)
        .bind(transaction_id)
        .bind(&idempotency_key)
        .bind(&metadata)
        .execute(&mut *tx)
        .await?;

        if row.fee > Decimal::ZERO {
            let fee_key = format!("withdrawal-fee:{transaction_id}");
            sqlx::query(
                r#"
                INSERT INTO ledger_entries (
                    journal_id, wallet_id, asset, chain, amount, entry_type,
                    transaction_id, idempotency_key, metadata
                )
                VALUES (gen_random_uuid(), $1, $2, $3, $4, 'fee', $5, $6, $7)
                "#,
            )
            .bind(row.wallet_id)
            .bind(&row.asset)
            .bind(&row.chain)
            .bind(-row.fee)
            .bind(transaction_id)
            .bind(&fee_key)
            .bind(&metadata)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(Some(transaction.try_into()?))
    }

    pub async fn fail_withdrawal(
        &self,
        transaction_id: Uuid,
        tx_hash: &str,
    ) -> WalletResult<Option<WalletTransaction>> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, WithdrawalLockRow>(
            r#"
            SELECT t.id, t.wallet_id, t.asset, t.chain, t.amount, t.fee, t.status AS tx_status
            FROM wallet_transactions t
            WHERE t.id = $1 AND t.type = 'withdrawal' AND t.tx_hash = $2
            FOR UPDATE OF t
            "#,
        )
        .bind(transaction_id)
        .bind(tx_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };

        if row.tx_status != "confirming" {
            tx.commit().await?;
            return Ok(None);
        }

        let hold = row.amount + row.fee;
        sqlx::query(
            r#"
            UPDATE wallet_balances
            SET available = available + $4, locked = locked - $4, updated_at = NOW()
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3
            "#,
        )
        .bind(row.wallet_id)
        .bind(&row.asset)
        .bind(&row.chain)
        .bind(hold)
        .execute(&mut *tx)
        .await?;

        let transaction = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            UPDATE wallet_transactions
            SET status = 'failed', updated_at = NOW()
            WHERE id = $1
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(transaction_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(transaction.try_into()?))
    }

    /// Atomic double-entry transfer between two wallets (same asset/chain).
    pub async fn internal_transfer(
        &self,
        req: &InternalTransferRequest,
    ) -> WalletResult<InternalTransferResponse> {
        let mut tx = self.pool.begin().await?;
        let result = self
            .internal_transfer_tx(&mut tx, req)
            .await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Same as `internal_transfer` but participates in an outer transaction (e.g. RSC Pay).
    pub async fn internal_transfer_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        req: &InternalTransferRequest,
    ) -> WalletResult<InternalTransferResponse> {
        if req.from_wallet_id == req.to_wallet_id {
            return Err(WalletError::Validation(
                "cannot transfer to the same wallet".into(),
            ));
        }
        if req.amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        if let Some(existing_journal) = sqlx::query_scalar::<_, Uuid>(
            r#"SELECT journal_id FROM ledger_entries WHERE idempotency_key = $1 LIMIT 1"#,
        )
        .bind(&req.idempotency_key)
        .fetch_optional(&mut **tx)
        .await?
        {
            let from_balance = self
                .get_balance_tx(tx, req.from_wallet_id, &req.asset, &req.chain)
                .await?;
            let to_balance = self
                .get_balance_tx(tx, req.to_wallet_id, &req.asset, &req.chain)
                .await?;
            return Ok(InternalTransferResponse {
                journal_id: existing_journal,
                from_balance,
                to_balance,
                idempotent_replay: true,
            });
        }

        let (first, second) = Self::order_wallet_ids(req.from_wallet_id, req.to_wallet_id);
        for wallet_id in [first, second] {
            sqlx::query("SELECT id FROM wallets WHERE id = $1 FOR UPDATE")
                .bind(wallet_id)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or(WalletError::WalletNotFound)?;
        }

        sqlx::query(
            r#"
            INSERT INTO wallet_balances (wallet_id, asset, chain)
            VALUES ($1, $2, $3), ($4, $2, $3)
            ON CONFLICT (wallet_id, asset, chain) DO NOTHING
            "#,
        )
        .bind(req.from_wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.to_wallet_id)
        .execute(&mut **tx)
        .await?;

        let updated = sqlx::query(
            r#"
            UPDATE wallet_balances
            SET available = available - $4, total = total - $4, updated_at = NOW()
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3 AND available >= $4
            "#,
        )
        .bind(req.from_wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .execute(&mut **tx)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(WalletError::InsufficientBalance);
        }

        sqlx::query(
            r#"
            UPDATE wallet_balances
            SET available = available + $4, total = total + $4, updated_at = NOW()
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3
            "#,
        )
        .bind(req.to_wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .execute(&mut **tx)
        .await?;

        let journal_id = Uuid::new_v4();
        let metadata = req
            .metadata
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));

        let transfer_out = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            INSERT INTO wallet_transactions (
                wallet_id, type, asset, chain, amount, fee, status, metadata
            )
            VALUES ($1, 'transfer_out', $2, $3, $4, 0, 'confirmed', $5)
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(req.from_wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .bind(&metadata)
        .fetch_one(&mut **tx)
        .await?;

        let transfer_in = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            INSERT INTO wallet_transactions (
                wallet_id, type, asset, chain, amount, fee, status, metadata
            )
            VALUES ($1, 'transfer_in', $2, $3, $4, 0, 'confirmed', $5)
            RETURNING id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                      status, tx_hash, from_address, to_address, confirmations,
                      created_at, updated_at
            "#,
        )
        .bind(req.to_wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .bind(&metadata)
        .fetch_one(&mut **tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO ledger_entries (
                journal_id, wallet_id, asset, chain, amount, entry_type,
                related_wallet_id, transaction_id, idempotency_key, metadata
            )
            VALUES ($1, $2, $3, $4, $5, 'internal_transfer', $6, $7, $8, $9)
            "#,
        )
        .bind(journal_id)
        .bind(req.from_wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(-req.amount)
        .bind(req.to_wallet_id)
        .bind(transfer_out.id)
        .bind(format!("{}:debit", req.idempotency_key))
        .bind(&metadata)
        .execute(&mut **tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO ledger_entries (
                journal_id, wallet_id, asset, chain, amount, entry_type,
                related_wallet_id, transaction_id, idempotency_key, metadata
            )
            VALUES ($1, $2, $3, $4, $5, 'internal_transfer', $6, $7, $8, $9)
            "#,
        )
        .bind(journal_id)
        .bind(req.to_wallet_id)
        .bind(&req.asset)
        .bind(&req.chain)
        .bind(req.amount)
        .bind(req.from_wallet_id)
        .bind(transfer_in.id)
        .bind(&req.idempotency_key)
        .bind(&metadata)
        .execute(&mut **tx)
        .await?;

        let from_balance = self
            .get_balance_tx(tx, req.from_wallet_id, &req.asset, &req.chain)
            .await?;
        let to_balance = self
            .get_balance_tx(tx, req.to_wallet_id, &req.asset, &req.chain)
            .await?;

        Ok(InternalTransferResponse {
            journal_id,
            from_balance,
            to_balance,
            idempotent_replay: false,
        })
    }

    async fn get_balance_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        wallet_id: Uuid,
        asset: &str,
        chain: &str,
    ) -> WalletResult<WalletBalance> {
        let row = sqlx::query_as::<_, WalletBalanceRow>(
            r#"
            SELECT wallet_id, asset, chain, available, total, locked, updated_at
            FROM wallet_balances
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3
            "#,
        )
        .bind(wallet_id)
        .bind(asset)
        .bind(chain)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row.into())
    }

    pub async fn health_check(&self) -> WalletResult<bool> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(true)
    }

    fn order_wallet_ids(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }

    async fn find_deposit_by_idempotency_key(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        idempotency_key: &str,
    ) -> WalletResult<Option<RecordDepositResponse>> {
        let Some(ledger_entry) = sqlx::query_as::<_, LedgerEntryRow>(
            r#"
            SELECT id, journal_id, wallet_id, asset, chain, amount, entry_type,
                   related_wallet_id, transaction_id, idempotency_key, metadata, created_at
            FROM ledger_entries
            WHERE idempotency_key = $1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&mut **tx)
        .await?
        else {
            return Ok(None);
        };

        let transaction_id = ledger_entry.transaction_id.ok_or_else(|| {
            WalletError::Internal(anyhow!(
                "ledger entry with idempotency key has no transaction_id"
            ))
        })?;

        let transaction = sqlx::query_as::<_, WalletTransactionRow>(
            r#"
            SELECT id, wallet_id, type AS tx_type, asset, chain, amount, fee,
                   status, tx_hash, from_address, to_address, confirmations,
                   created_at, updated_at
            FROM wallet_transactions
            WHERE id = $1
            "#,
        )
        .bind(transaction_id)
        .fetch_one(&mut **tx)
        .await?;

        let balance = sqlx::query_as::<_, WalletBalanceRow>(
            r#"
            SELECT wallet_id, asset, chain, available, total, locked, updated_at
            FROM wallet_balances
            WHERE wallet_id = $1 AND asset = $2 AND chain = $3
            "#,
        )
        .bind(ledger_entry.wallet_id)
        .bind(&ledger_entry.asset)
        .bind(&ledger_entry.chain)
        .fetch_one(&mut **tx)
        .await?;

        Ok(Some(RecordDepositResponse {
            transaction: transaction.try_into()?,
            balance: balance.into(),
            ledger_entry: ledger_entry.try_into()?,
            idempotent_replay: true,
        }))
    }
}

#[derive(sqlx::FromRow)]
struct WalletRow {
    id: Uuid,
    user_id: Uuid,
    label: Option<String>,
    is_default: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<WalletRow> for Wallet {
    fn from(row: WalletRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            label: row.label,
            is_default: row.is_default,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WalletBalanceRow {
    wallet_id: Uuid,
    asset: String,
    chain: String,
    available: Decimal,
    total: Decimal,
    locked: Decimal,
    updated_at: DateTime<Utc>,
}

impl From<WalletBalanceRow> for WalletBalance {
    fn from(row: WalletBalanceRow) -> Self {
        Self {
            wallet_id: row.wallet_id,
            asset: Asset(row.asset),
            chain: Chain(row.chain),
            available: row.available,
            total: row.total,
            locked: row.locked,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WithdrawalLockRow {
    id: Uuid,
    wallet_id: Uuid,
    asset: String,
    chain: String,
    amount: Decimal,
    fee: Decimal,
    tx_status: String,
}

#[derive(sqlx::FromRow)]
struct DepositTargetRow {
    wallet_id: Uuid,
    asset: String,
    chain: String,
}

impl From<DepositTargetRow> for DepositTarget {
    fn from(row: DepositTargetRow) -> Self {
        Self {
            wallet_id: row.wallet_id,
            asset: row.asset,
            chain: row.chain,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WalletAddressRow {
    id: Uuid,
    wallet_id: Uuid,
    asset: String,
    chain: String,
    address: String,
    derivation_path: Option<String>,
    is_used: bool,
    created_at: DateTime<Utc>,
}

impl From<WalletAddressRow> for WalletAddress {
    fn from(row: WalletAddressRow) -> Self {
        Self {
            id: row.id,
            wallet_id: row.wallet_id,
            asset: Asset(row.asset),
            chain: Chain(row.chain),
            address: row.address,
            derivation_path: row.derivation_path,
            is_used: row.is_used,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WalletTransactionRow {
    id: Uuid,
    wallet_id: Uuid,
    tx_type: String,
    asset: String,
    chain: String,
    amount: Decimal,
    fee: Decimal,
    status: String,
    tx_hash: Option<String>,
    from_address: Option<String>,
    to_address: Option<String>,
    confirmations: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<WalletTransactionRow> for WalletTransaction {
    type Error = WalletError;

    fn try_from(row: WalletTransactionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            wallet_id: row.wallet_id,
            r#type: parse_transaction_type(&row.tx_type)?,
            asset: Asset(row.asset),
            chain: Chain(row.chain),
            amount: row.amount,
            fee: row.fee,
            status: parse_transaction_status(&row.status)?,
            tx_hash: row.tx_hash,
            from_address: row.from_address,
            to_address: row.to_address,
            confirmations: row.confirmations,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct LedgerEntryRow {
    id: Uuid,
    journal_id: Uuid,
    wallet_id: Uuid,
    asset: String,
    chain: String,
    amount: Decimal,
    entry_type: String,
    related_wallet_id: Option<Uuid>,
    transaction_id: Option<Uuid>,
    idempotency_key: Option<String>,
    metadata: Value,
    created_at: DateTime<Utc>,
}

impl TryFrom<LedgerEntryRow> for LedgerEntry {
    type Error = WalletError;

    fn try_from(row: LedgerEntryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            journal_id: row.journal_id,
            wallet_id: row.wallet_id,
            asset: Asset(row.asset),
            chain: Chain(row.chain),
            amount: row.amount,
            entry_type: parse_ledger_entry_type(&row.entry_type)?,
            related_wallet_id: row.related_wallet_id,
            transaction_id: row.transaction_id,
            idempotency_key: row.idempotency_key,
            metadata: row.metadata,
            created_at: row.created_at,
        })
    }
}

fn parse_transaction_type(value: &str) -> WalletResult<TransactionType> {
    match value {
        "deposit" => Ok(TransactionType::Deposit),
        "withdrawal" => Ok(TransactionType::Withdrawal),
        "transfer_in" => Ok(TransactionType::TransferIn),
        "transfer_out" => Ok(TransactionType::TransferOut),
        "fee" => Ok(TransactionType::Fee),
        other => Err(WalletError::Internal(anyhow!(
            "unknown wallet transaction type: {other}"
        ))),
    }
}

fn parse_transaction_status(value: &str) -> WalletResult<TransactionStatus> {
    match value {
        "pending" => Ok(TransactionStatus::Pending),
        "confirming" => Ok(TransactionStatus::Confirming),
        "confirmed" => Ok(TransactionStatus::Confirmed),
        "failed" => Ok(TransactionStatus::Failed),
        "cancelled" => Ok(TransactionStatus::Cancelled),
        other => Err(WalletError::Internal(anyhow!(
            "unknown wallet transaction status: {other}"
        ))),
    }
}

fn parse_ledger_entry_type(value: &str) -> WalletResult<LedgerEntryType> {
    match value {
        "deposit" => Ok(LedgerEntryType::Deposit),
        "withdrawal" => Ok(LedgerEntryType::Withdrawal),
        "internal_transfer" => Ok(LedgerEntryType::InternalTransfer),
        "fee" => Ok(LedgerEntryType::Fee),
        "adjustment" => Ok(LedgerEntryType::Adjustment),
        other => Err(WalletError::Internal(anyhow!(
            "unknown ledger entry type: {other}"
        ))),
    }
}
