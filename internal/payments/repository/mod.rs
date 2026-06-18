//! Payments repository — merchants, invoices, payments, settlements.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::payments::error::{PaymentError, PaymentResult};
use crate::internal::payments::models::{
    CreateInvoiceRequest, InvoiceStatus, Merchant, MerchantStatus, Payment, PaymentInvoice,
    PaymentMethod, PaymentStatus, RegisterMerchantRequest, Settlement, SettlementStatus,
};
use crate::internal::wallets::error::WalletError;
use crate::internal::wallets::models::InternalTransferRequest;
use crate::internal::wallets::repository::WalletRepository;

#[derive(Clone)]
pub struct PaymentRepository {
    pool: PgPool,
}

impl PaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ==================== Merchants ====================

    pub async fn create_merchant(
        &self,
        owner_user_id: Uuid,
        req: &RegisterMerchantRequest,
        settlement_asset: &str,
        settlement_chain: &str,
    ) -> PaymentResult<Merchant> {
        let row = sqlx::query_as::<_, MerchantRow>(
            r#"
            INSERT INTO merchants (
                owner_user_id, wallet_id, display_name, legal_name,
                settlement_asset, settlement_chain
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, owner_user_id, wallet_id, display_name, legal_name, status,
                      settlement_asset, settlement_chain, created_at, updated_at
            "#,
        )
        .bind(owner_user_id)
        .bind(req.wallet_id)
        .bind(&req.display_name)
        .bind(&req.legal_name)
        .bind(settlement_asset)
        .bind(settlement_chain)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db) = &e {
                if db.constraint() == Some("idx_merchants_owner_display") {
                    return PaymentError::Conflict("merchant display name already exists".into());
                }
            }
            PaymentError::Database(e)
        })?;

        Ok(row.into())
    }

    pub async fn find_merchant_by_owner(
        &self,
        owner_user_id: Uuid,
    ) -> PaymentResult<Option<Merchant>> {
        let row = sqlx::query_as::<_, MerchantRow>(
            r#"
            SELECT id, owner_user_id, wallet_id, display_name, legal_name, status,
                   settlement_asset, settlement_chain, created_at, updated_at
            FROM merchants
            WHERE owner_user_id = $1 AND status = 'active'
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(owner_user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn find_merchant_by_id(&self, id: Uuid) -> PaymentResult<Option<Merchant>> {
        let row = sqlx::query_as::<_, MerchantRow>(
            r#"
            SELECT id, owner_user_id, wallet_id, display_name, legal_name, status,
                   settlement_asset, settlement_chain, created_at, updated_at
            FROM merchants WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn merchant_owned_by(
        &self,
        merchant_id: Uuid,
        owner_user_id: Uuid,
    ) -> PaymentResult<bool> {
        let found: Option<Uuid> =
            sqlx::query_scalar(r#"SELECT id FROM merchants WHERE id = $1 AND owner_user_id = $2"#)
                .bind(merchant_id)
                .bind(owner_user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(found.is_some())
    }

    // ==================== Invoices ====================

    pub async fn create_invoice(
        &self,
        merchant_id: Uuid,
        reference_code: &str,
        req: &CreateInvoiceRequest,
        asset: &str,
        chain: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> PaymentResult<PaymentInvoice> {
        if let Some(key) = &req.idempotency_key {
            if let Some(existing) = sqlx::query_as::<_, InvoiceRow>(
                r#"
                SELECT id, merchant_id, reference_code, amount, asset, chain, description,
                       status, expires_at, paid_at, created_at, updated_at
                FROM payment_invoices
                WHERE merchant_id = $1 AND idempotency_key = $2
                "#,
            )
            .bind(merchant_id)
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            {
                return Ok(existing.into());
            }
        }

        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            INSERT INTO payment_invoices (
                merchant_id, reference_code, amount, asset, chain, description,
                status, expires_at, idempotency_key
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8)
            RETURNING id, merchant_id, reference_code, amount, asset, chain, description,
                      status, expires_at, paid_at, created_at, updated_at
            "#,
        )
        .bind(merchant_id)
        .bind(reference_code)
        .bind(req.amount)
        .bind(asset)
        .bind(chain)
        .bind(&req.description)
        .bind(expires_at)
        .bind(&req.idempotency_key)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn find_invoice_by_reference(
        &self,
        reference_code: &str,
    ) -> PaymentResult<Option<PaymentInvoice>> {
        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT id, merchant_id, reference_code, amount, asset, chain, description,
                   status, expires_at, paid_at, created_at, updated_at
            FROM payment_invoices WHERE reference_code = $1
            "#,
        )
        .bind(reference_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn find_invoice_by_id(&self, id: Uuid) -> PaymentResult<Option<PaymentInvoice>> {
        let row = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT id, merchant_id, reference_code, amount, asset, chain, description,
                   status, expires_at, paid_at, created_at, updated_at
            FROM payment_invoices WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn get_invoice_public_view(
        &self,
        reference_code: &str,
    ) -> PaymentResult<Option<(PaymentInvoice, String)>> {
        let row = sqlx::query_as::<_, InvoicePublicRow>(
            r#"
            SELECT i.id, i.merchant_id, i.reference_code, i.amount, i.asset, i.chain,
                   i.description, i.status, i.expires_at, i.paid_at, i.created_at, i.updated_at,
                   m.display_name AS merchant_display_name
            FROM payment_invoices i
            INNER JOIN merchants m ON m.id = i.merchant_id
            WHERE i.reference_code = $1
            "#,
        )
        .bind(reference_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let invoice: PaymentInvoice = InvoiceRow {
                id: r.id,
                merchant_id: r.merchant_id,
                reference_code: r.reference_code,
                amount: r.amount,
                asset: r.asset,
                chain: r.chain,
                description: r.description,
                status: r.status,
                expires_at: r.expires_at,
                paid_at: r.paid_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }
            .into();
            (invoice, r.merchant_display_name)
        }))
    }

    // ==================== Payments (engine) ====================

    pub async fn complete_payment(
        &self,
        invoice_id: Uuid,
        payer_user_id: Uuid,
        amount: Decimal,
        fee: Decimal,
        method: PaymentMethod,
        idempotency_key: &str,
    ) -> PaymentResult<(Payment, PaymentInvoice, bool)> {
        let mut tx = self.pool.begin().await?;

        if let Some(existing) = sqlx::query_as::<_, PaymentRow>(
            r#"
            SELECT id, invoice_id, payer_user_id, amount, fee, method, status,
                   idempotency_key, wallet_journal_id, created_at, updated_at
            FROM payments WHERE idempotency_key = $1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await?
        {
            let invoice = sqlx::query_as::<_, InvoiceRow>(
                r#"
                SELECT id, merchant_id, reference_code, amount, asset, chain, description,
                       status, expires_at, paid_at, created_at, updated_at
                FROM payment_invoices WHERE id = $1
                "#,
            )
            .bind(existing.invoice_id)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok((existing.try_into()?, invoice.into(), true));
        }

        let invoice = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT id, merchant_id, reference_code, amount, asset, chain, description,
                   status, expires_at, paid_at, created_at, updated_at
            FROM payment_invoices
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(invoice_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(PaymentError::InvoiceNotFound)?;

        if invoice.status == "paid" {
            return Err(PaymentError::InvoiceAlreadyPaid);
        }
        if invoice.status == "expired" || invoice.status == "cancelled" {
            return Err(PaymentError::InvoiceExpired);
        }
        if let Some(exp) = invoice.expires_at {
            if exp < Utc::now() {
                sqlx::query(
                    r#"UPDATE payment_invoices SET status = 'expired', updated_at = NOW() WHERE id = $1"#,
                )
                .bind(invoice_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Err(PaymentError::InvoiceExpired);
            }
        }

        let method_str = payment_method_str(method);
        let payment = sqlx::query_as::<_, PaymentRow>(
            r#"
            INSERT INTO payments (
                invoice_id, payer_user_id, amount, fee, method, status, idempotency_key
            )
            VALUES ($1, $2, $3, $4, $5, 'completed', $6)
            RETURNING id, invoice_id, payer_user_id, amount, fee, method, status,
                      idempotency_key, wallet_journal_id, created_at, updated_at
            "#,
        )
        .bind(invoice_id)
        .bind(payer_user_id)
        .bind(amount)
        .bind(fee)
        .bind(method_str)
        .bind(idempotency_key)
        .fetch_one(&mut *tx)
        .await?;

        let updated_invoice = sqlx::query_as::<_, InvoiceRow>(
            r#"
            UPDATE payment_invoices
            SET status = 'paid', paid_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING id, merchant_id, reference_code, amount, asset, chain, description,
                      status, expires_at, paid_at, created_at, updated_at
            "#,
        )
        .bind(invoice_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((payment.try_into()?, updated_invoice.into(), false))
    }

    /// Completes a payment and moves funds payer → settlement clearing wallet in one DB transaction.
    pub async fn complete_payment_with_wallet_transfer(
        &self,
        wallet_repo: &WalletRepository,
        invoice_id: Uuid,
        payer_user_id: Uuid,
        payer_wallet_id: Uuid,
        clearing_wallet_id: Uuid,
        amount: Decimal,
        fee: Decimal,
        asset: &str,
        chain: &str,
        method: PaymentMethod,
        idempotency_key: &str,
    ) -> PaymentResult<(Payment, PaymentInvoice, bool, bool)> {
        let mut tx = self.pool.begin().await?;

        if let Some(existing) = sqlx::query_as::<_, PaymentRow>(
            r#"
            SELECT id, invoice_id, payer_user_id, amount, fee, method, status,
                   idempotency_key, wallet_journal_id, created_at, updated_at
            FROM payments WHERE idempotency_key = $1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await?
        {
            let invoice = sqlx::query_as::<_, InvoiceRow>(
                r#"
                SELECT id, merchant_id, reference_code, amount, asset, chain, description,
                       status, expires_at, paid_at, created_at, updated_at
                FROM payment_invoices WHERE id = $1
                "#,
            )
            .bind(existing.invoice_id)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok((existing.try_into()?, invoice.into(), true, true));
        }

        let invoice = sqlx::query_as::<_, InvoiceRow>(
            r#"
            SELECT id, merchant_id, reference_code, amount, asset, chain, description,
                   status, expires_at, paid_at, created_at, updated_at
            FROM payment_invoices
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(invoice_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(PaymentError::InvoiceNotFound)?;

        if invoice.status == "paid" {
            return Err(PaymentError::InvoiceAlreadyPaid);
        }
        if invoice.status == "expired" || invoice.status == "cancelled" {
            return Err(PaymentError::InvoiceExpired);
        }
        if let Some(exp) = invoice.expires_at {
            if exp < Utc::now() {
                sqlx::query(
                    r#"UPDATE payment_invoices SET status = 'expired', updated_at = NOW() WHERE id = $1"#,
                )
                .bind(invoice_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Err(PaymentError::InvoiceExpired);
            }
        }

        if invoice.asset != asset || invoice.chain != chain {
            return Err(PaymentError::Validation(
                "invoice asset/chain mismatch".into(),
            ));
        }
        if invoice.amount != amount {
            return Err(PaymentError::InvalidAmount);
        }

        let transfer_key = format!("payment-transfer:{idempotency_key}");
        let transfer_meta = serde_json::json!({
            "invoice_id": invoice_id,
            "payer_user_id": payer_user_id,
            "payment_idempotency_key": idempotency_key,
        });

        let transfer = wallet_repo
            .internal_transfer_tx(
                &mut tx,
                &InternalTransferRequest {
                    from_wallet_id: payer_wallet_id,
                    to_wallet_id: clearing_wallet_id,
                    asset: asset.to_string(),
                    chain: chain.to_string(),
                    amount,
                    idempotency_key: transfer_key,
                    metadata: Some(transfer_meta),
                },
            )
            .await
            .map_err(map_wallet_error)?;

        let method_str = payment_method_str(method);
        let payment = sqlx::query_as::<_, PaymentRow>(
            r#"
            INSERT INTO payments (
                invoice_id, payer_user_id, amount, fee, method, status,
                idempotency_key, wallet_journal_id
            )
            VALUES ($1, $2, $3, $4, $5, 'completed', $6, $7)
            RETURNING id, invoice_id, payer_user_id, amount, fee, method, status,
                      idempotency_key, wallet_journal_id, created_at, updated_at
            "#,
        )
        .bind(invoice_id)
        .bind(payer_user_id)
        .bind(amount)
        .bind(fee)
        .bind(method_str)
        .bind(idempotency_key)
        .bind(transfer.journal_id)
        .fetch_one(&mut *tx)
        .await?;

        let updated_invoice = sqlx::query_as::<_, InvoiceRow>(
            r#"
            UPDATE payment_invoices
            SET status = 'paid', paid_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING id, merchant_id, reference_code, amount, asset, chain, description,
                      status, expires_at, paid_at, created_at, updated_at
            "#,
        )
        .bind(invoice_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((
            payment.try_into()?,
            updated_invoice.into(),
            false,
            transfer.idempotent_replay,
        ))
    }

    pub async fn list_payments_for_merchant(
        &self,
        merchant_id: Uuid,
        limit: i64,
    ) -> PaymentResult<Vec<Payment>> {
        let rows = sqlx::query_as::<_, PaymentRow>(
            r#"
            SELECT p.id, p.invoice_id, p.payer_user_id, p.amount, p.fee, p.method, p.status,
                   p.idempotency_key, p.wallet_journal_id, p.created_at, p.updated_at
            FROM payments p
            INNER JOIN payment_invoices i ON i.id = p.invoice_id
            WHERE i.merchant_id = $1 AND p.status = 'completed'
            ORDER BY p.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(merchant_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    // ==================== Settlements ====================

    pub async fn create_settlement(
        &self,
        merchant_id: Uuid,
        asset: &str,
        chain: &str,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> PaymentResult<(Settlement, usize)> {
        let mut tx = self.pool.begin().await?;

        let rows = sqlx::query_as::<_, UnsettledPaymentRow>(
            r#"
            SELECT p.id, p.amount
            FROM payments p
            INNER JOIN payment_invoices i ON i.id = p.invoice_id
            WHERE i.merchant_id = $1
              AND p.status = 'completed'
              AND NOT EXISTS (
                  SELECT 1 FROM settlement_items si WHERE si.payment_id = p.id
              )
              AND ($2::timestamptz IS NULL OR p.created_at >= $2)
              AND ($3::timestamptz IS NULL OR p.created_at <= $3)
            FOR UPDATE OF p
            "#,
        )
        .bind(merchant_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&mut *tx)
        .await?;

        if rows.is_empty() {
            return Err(PaymentError::Validation(
                "no unsettled payments for settlement".into(),
            ));
        }

        let total: Decimal = rows.iter().map(|r| r.amount).sum();

        let settlement = sqlx::query_as::<_, SettlementRow>(
            r#"
            INSERT INTO settlements (
                merchant_id, amount, asset, chain, status, period_start, period_end
            )
            VALUES ($1, $2, $3, $4, 'pending', $5, $6)
            RETURNING id, merchant_id, amount, asset, chain, status, wallet_journal_id,
                      destination_wallet_id, period_start, period_end, created_at, updated_at
            "#,
        )
        .bind(merchant_id)
        .bind(total)
        .bind(asset)
        .bind(chain)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&mut *tx)
        .await?;

        for row in &rows {
            sqlx::query(
                r#"
                INSERT INTO settlement_items (settlement_id, payment_id, amount)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(settlement.id)
            .bind(row.id)
            .bind(row.amount)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok((settlement.into(), rows.len()))
    }

    /// Batches unsettled payments and transfers clearing → merchant wallet in one transaction.
    pub async fn create_settlement_with_wallet_transfer(
        &self,
        wallet_repo: &WalletRepository,
        merchant_id: Uuid,
        merchant_wallet_id: Uuid,
        clearing_wallet_id: Uuid,
        asset: &str,
        chain: &str,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
        idempotency_key: &str,
    ) -> PaymentResult<(Settlement, usize, bool)> {
        let mut tx = self.pool.begin().await?;

        if let Some(existing) = sqlx::query_as::<_, SettlementRow>(
            r#"
            SELECT id, merchant_id, amount, asset, chain, status, wallet_journal_id,
                   destination_wallet_id, period_start, period_end, created_at, updated_at
            FROM settlements
            WHERE merchant_id = $1
              AND metadata->>'idempotency_key' = $2
            "#,
        )
        .bind(merchant_id)
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await?
        {
            let count: i64 = sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM settlement_items WHERE settlement_id = $1"#,
            )
            .bind(existing.id)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok((existing.into(), count as usize, true));
        }

        let rows = sqlx::query_as::<_, UnsettledPaymentRow>(
            r#"
            SELECT p.id, p.amount
            FROM payments p
            INNER JOIN payment_invoices i ON i.id = p.invoice_id
            WHERE i.merchant_id = $1
              AND p.status = 'completed'
              AND p.wallet_journal_id IS NOT NULL
              AND NOT EXISTS (
                  SELECT 1 FROM settlement_items si WHERE si.payment_id = p.id
              )
              AND ($2::timestamptz IS NULL OR p.created_at >= $2)
              AND ($3::timestamptz IS NULL OR p.created_at <= $3)
            FOR UPDATE OF p
            "#,
        )
        .bind(merchant_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&mut *tx)
        .await?;

        if rows.is_empty() {
            return Err(PaymentError::Validation(
                "no unsettled payments for settlement".into(),
            ));
        }

        let total: Decimal = rows.iter().map(|r| r.amount).sum();

        let settlement = sqlx::query_as::<_, SettlementRow>(
            r#"
            INSERT INTO settlements (
                merchant_id, amount, asset, chain, status,
                period_start, period_end, destination_wallet_id, metadata
            )
            VALUES ($1, $2, $3, $4, 'processing', $5, $6, $7,
                    jsonb_build_object('idempotency_key', $8::text))
            RETURNING id, merchant_id, amount, asset, chain, status, wallet_journal_id,
                      destination_wallet_id, period_start, period_end, created_at, updated_at
            "#,
        )
        .bind(merchant_id)
        .bind(total)
        .bind(asset)
        .bind(chain)
        .bind(period_start)
        .bind(period_end)
        .bind(merchant_wallet_id)
        .bind(idempotency_key)
        .fetch_one(&mut *tx)
        .await?;

        for row in &rows {
            sqlx::query(
                r#"
                INSERT INTO settlement_items (settlement_id, payment_id, amount)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(settlement.id)
            .bind(row.id)
            .bind(row.amount)
            .execute(&mut *tx)
            .await?;
        }

        let transfer_key = format!("settlement-transfer:{idempotency_key}");
        let transfer_meta = serde_json::json!({
            "settlement_id": settlement.id,
            "merchant_id": merchant_id,
            "payment_count": rows.len(),
        });

        let transfer = wallet_repo
            .internal_transfer_tx(
                &mut tx,
                &InternalTransferRequest {
                    from_wallet_id: clearing_wallet_id,
                    to_wallet_id: merchant_wallet_id,
                    asset: asset.to_string(),
                    chain: chain.to_string(),
                    amount: total,
                    idempotency_key: transfer_key,
                    metadata: Some(transfer_meta),
                },
            )
            .await
            .map_err(map_wallet_error)?;

        let completed = sqlx::query_as::<_, SettlementRow>(
            r#"
            UPDATE settlements
            SET status = 'completed',
                wallet_journal_id = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, merchant_id, amount, asset, chain, status, wallet_journal_id,
                      destination_wallet_id, period_start, period_end, created_at, updated_at
            "#,
        )
        .bind(settlement.id)
        .bind(transfer.journal_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((completed.into(), rows.len(), transfer.idempotent_replay))
    }

    pub async fn list_settlements(
        &self,
        merchant_id: Uuid,
        limit: i64,
    ) -> PaymentResult<Vec<Settlement>> {
        let rows = sqlx::query_as::<_, SettlementRow>(
            r#"
            SELECT id, merchant_id, amount, asset, chain, status, wallet_journal_id,
                   destination_wallet_id, period_start, period_end, created_at, updated_at
            FROM settlements
            WHERE merchant_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(merchant_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let settlements: Vec<Settlement> = rows.into_iter().map(Into::into).collect();
        Ok(settlements)
    }
}

// ==================== Row types ====================

#[derive(sqlx::FromRow)]
struct MerchantRow {
    id: Uuid,
    owner_user_id: Uuid,
    wallet_id: Option<Uuid>,
    display_name: String,
    legal_name: Option<String>,
    status: String,
    settlement_asset: String,
    settlement_chain: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<MerchantRow> for Merchant {
    fn from(row: MerchantRow) -> Self {
        Self {
            id: row.id,
            owner_user_id: row.owner_user_id,
            wallet_id: row.wallet_id,
            display_name: row.display_name,
            legal_name: row.legal_name,
            status: parse_merchant_status(&row.status),
            settlement_asset: row.settlement_asset,
            settlement_chain: row.settlement_chain,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct InvoiceRow {
    id: Uuid,
    merchant_id: Uuid,
    reference_code: String,
    amount: Decimal,
    asset: String,
    chain: String,
    description: Option<String>,
    status: String,
    expires_at: Option<DateTime<Utc>>,
    paid_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<InvoiceRow> for PaymentInvoice {
    fn from(row: InvoiceRow) -> Self {
        Self {
            id: row.id,
            merchant_id: row.merchant_id,
            reference_code: row.reference_code,
            amount: row.amount,
            asset: row.asset,
            chain: row.chain,
            description: row.description,
            status: parse_invoice_status(&row.status),
            expires_at: row.expires_at,
            paid_at: row.paid_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct InvoicePublicRow {
    id: Uuid,
    merchant_id: Uuid,
    reference_code: String,
    amount: Decimal,
    asset: String,
    chain: String,
    description: Option<String>,
    status: String,
    expires_at: Option<DateTime<Utc>>,
    paid_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    merchant_display_name: String,
}

impl From<InvoicePublicRow> for PaymentInvoice {
    fn from(row: InvoicePublicRow) -> Self {
        Self {
            id: row.id,
            merchant_id: row.merchant_id,
            reference_code: row.reference_code,
            amount: row.amount,
            asset: row.asset,
            chain: row.chain,
            description: row.description,
            status: parse_invoice_status(&row.status),
            expires_at: row.expires_at,
            paid_at: row.paid_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PaymentRow {
    id: Uuid,
    invoice_id: Uuid,
    payer_user_id: Uuid,
    amount: Decimal,
    fee: Decimal,
    method: String,
    status: String,
    idempotency_key: String,
    wallet_journal_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PaymentRow> for Payment {
    type Error = PaymentError;

    fn try_from(row: PaymentRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            invoice_id: row.invoice_id,
            payer_user_id: row.payer_user_id,
            amount: row.amount,
            fee: row.fee,
            method: parse_payment_method(&row.method)?,
            status: parse_payment_status(&row.status)?,
            idempotency_key: row.idempotency_key,
            wallet_journal_id: row.wallet_journal_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn map_wallet_error(err: WalletError) -> PaymentError {
    match err {
        WalletError::InsufficientBalance => PaymentError::InsufficientBalance,
        WalletError::InvalidAmount => PaymentError::InvalidAmount,
        WalletError::Validation(msg) => PaymentError::Validation(msg),
        WalletError::Forbidden => PaymentError::Forbidden,
        WalletError::WalletNotFound => PaymentError::Validation("wallet not found".into()),
        WalletError::Database(e) => PaymentError::Database(e),
        WalletError::Internal(e) => PaymentError::Internal(e),
        other => PaymentError::Validation(other.to_string()),
    }
}

#[derive(sqlx::FromRow)]
struct SettlementRow {
    id: Uuid,
    merchant_id: Uuid,
    amount: Decimal,
    asset: String,
    chain: String,
    status: String,
    wallet_journal_id: Option<Uuid>,
    destination_wallet_id: Option<Uuid>,
    period_start: Option<DateTime<Utc>>,
    period_end: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SettlementRow> for Settlement {
    fn from(row: SettlementRow) -> Self {
        Self {
            id: row.id,
            merchant_id: row.merchant_id,
            amount: row.amount,
            asset: row.asset,
            chain: row.chain,
            status: parse_settlement_status(&row.status),
            wallet_journal_id: row.wallet_journal_id,
            destination_wallet_id: row.destination_wallet_id,
            period_start: row.period_start,
            period_end: row.period_end,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct UnsettledPaymentRow {
    id: Uuid,
    amount: Decimal,
}

fn payment_method_str(method: PaymentMethod) -> &'static str {
    match method {
        PaymentMethod::Qr => "qr",
        PaymentMethod::Instant => "instant",
        PaymentMethod::Invoice => "invoice",
        PaymentMethod::FiatRamp => "fiat_ramp",
        PaymentMethod::FiatTransak => "fiat_transak",
    }
}

fn parse_merchant_status(value: &str) -> MerchantStatus {
    match value {
        "pending" => MerchantStatus::Pending,
        "suspended" => MerchantStatus::Suspended,
        _ => MerchantStatus::Active,
    }
}

fn parse_invoice_status(value: &str) -> InvoiceStatus {
    match value {
        "draft" => InvoiceStatus::Draft,
        "paid" => InvoiceStatus::Paid,
        "expired" => InvoiceStatus::Expired,
        "cancelled" => InvoiceStatus::Cancelled,
        _ => InvoiceStatus::Pending,
    }
}

fn parse_payment_method(value: &str) -> PaymentResult<PaymentMethod> {
    match value {
        "qr" => Ok(PaymentMethod::Qr),
        "instant" => Ok(PaymentMethod::Instant),
        "invoice" => Ok(PaymentMethod::Invoice),
        "fiat_ramp" | "fiat-ramp" => Ok(PaymentMethod::FiatRamp),
        "fiat_transak" | "fiat-transak" => Ok(PaymentMethod::FiatTransak),
        "fiat" => Ok(PaymentMethod::FiatRamp),
        other => Err(PaymentError::Validation(format!(
            "unknown payment method: {other}"
        ))),
    }
}

fn parse_payment_status(value: &str) -> PaymentResult<PaymentStatus> {
    match value {
        "pending" => Ok(PaymentStatus::Pending),
        "completed" => Ok(PaymentStatus::Completed),
        "failed" => Ok(PaymentStatus::Failed),
        "refunded" => Ok(PaymentStatus::Refunded),
        other => Err(PaymentError::Validation(format!(
            "unknown payment status: {other}"
        ))),
    }
}

fn parse_settlement_status(value: &str) -> SettlementStatus {
    match value {
        "processing" => SettlementStatus::Processing,
        "completed" => SettlementStatus::Completed,
        "failed" => SettlementStatus::Failed,
        _ => SettlementStatus::Pending,
    }
}
