use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::providers::error::{ProviderError, ProviderResult};
use crate::internal::providers::models::{
    FiatConversionOrder, FiatOrderStatus, FiatProvider, ProviderQuote,
};

#[derive(Clone)]
pub struct FiatConversionRepository {
    pool: PgPool,
}

impl FiatConversionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: Uuid) -> ProviderResult<Option<FiatConversionOrder>> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            SELECT id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                   exchange_rate, checkout_url, wallet_address, idempotency_key,
                   completed_at, created_at, updated_at
            FROM fiat_conversion_orders WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn find_by_idempotency(
        &self,
        idempotency_key: &str,
    ) -> ProviderResult<Option<FiatConversionOrder>> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            SELECT id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                   exchange_rate, checkout_url, wallet_address, idempotency_key,
                   completed_at, created_at, updated_at
            FROM fiat_conversion_orders WHERE idempotency_key = $1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn find_by_external(
        &self,
        provider: FiatProvider,
        external_order_id: &str,
    ) -> ProviderResult<Option<FiatConversionOrder>> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            SELECT id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                   exchange_rate, checkout_url, wallet_address, idempotency_key,
                   completed_at, created_at, updated_at
            FROM fiat_conversion_orders
            WHERE provider = $1 AND external_order_id = $2
            "#,
        )
        .bind(provider.as_str())
        .bind(external_order_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn create_order(
        &self,
        user_id: Uuid,
        invoice_id: Option<Uuid>,
        provider: FiatProvider,
        quote: &ProviderQuote,
        wallet_address: &str,
        idempotency_key: &str,
        external_order_id: Option<&str>,
    ) -> ProviderResult<FiatConversionOrder> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            INSERT INTO fiat_conversion_orders (
                user_id, invoice_id, provider, external_order_id, status,
                fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                exchange_rate, checkout_url, wallet_address, idempotency_key, provider_metadata
            )
            VALUES (
                $1, $2, $3, $4, 'pending',
                $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14
            )
            RETURNING id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                      fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                      exchange_rate, checkout_url, wallet_address, idempotency_key,
                      completed_at, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(invoice_id)
        .bind(provider.as_str())
        .bind(external_order_id)
        .bind(&quote.fiat_currency)
        .bind(quote.fiat_amount)
        .bind(&quote.crypto_asset)
        .bind(&quote.crypto_chain)
        .bind(quote.crypto_amount)
        .bind(quote.exchange_rate)
        .bind(&quote.checkout_url)
        .bind(wallet_address)
        .bind(idempotency_key)
        .bind(&quote.raw)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db) = &e {
                if db.constraint() == Some("unique_fiat_order_idempotency") {
                    return ProviderError::Conflict("fiat order idempotency key already used".into());
                }
            }
            ProviderError::Database(e)
        })?;

        Ok(row.into())
    }

    pub async fn mark_processing(
        &self,
        order_id: Uuid,
        external_order_id: Option<&str>,
    ) -> ProviderResult<FiatConversionOrder> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            UPDATE fiat_conversion_orders
            SET status = 'processing',
                external_order_id = COALESCE($2, external_order_id),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                      fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                      exchange_rate, checkout_url, wallet_address, idempotency_key,
                      completed_at, created_at, updated_at
            "#,
        )
        .bind(order_id)
        .bind(external_order_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn mark_completed(
        &self,
        order_id: Uuid,
        payment_id: Option<Uuid>,
    ) -> ProviderResult<FiatConversionOrder> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            UPDATE fiat_conversion_orders
            SET status = 'completed',
                payment_id = COALESCE($2, payment_id),
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                      fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                      exchange_rate, checkout_url, wallet_address, idempotency_key,
                      completed_at, created_at, updated_at
            "#,
        )
        .bind(order_id)
        .bind(payment_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn attach_partner_reference(
        &self,
        order_id: Uuid,
        partner_reference: &str,
        checkout_url: Option<&str>,
    ) -> ProviderResult<FiatConversionOrder> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            UPDATE fiat_conversion_orders
            SET external_order_id = $2,
                checkout_url = COALESCE($3, checkout_url),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                      fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                      exchange_rate, checkout_url, wallet_address, idempotency_key,
                      completed_at, created_at, updated_at
            "#,
        )
        .bind(order_id)
        .bind(partner_reference)
        .bind(checkout_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn mark_failed(&self, order_id: Uuid) -> ProviderResult<FiatConversionOrder> {
        let row = sqlx::query_as::<_, FiatOrderRow>(
            r#"
            UPDATE fiat_conversion_orders
            SET status = 'failed', updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, invoice_id, payment_id, provider, external_order_id, status,
                      fiat_currency, fiat_amount, crypto_asset, crypto_chain, crypto_amount,
                      exchange_rate, checkout_url, wallet_address, idempotency_key,
                      completed_at, created_at, updated_at
            "#,
        )
        .bind(order_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }
}

#[derive(sqlx::FromRow)]
struct FiatOrderRow {
    id: Uuid,
    user_id: Uuid,
    invoice_id: Option<Uuid>,
    payment_id: Option<Uuid>,
    provider: String,
    external_order_id: Option<String>,
    status: String,
    fiat_currency: String,
    fiat_amount: Decimal,
    crypto_asset: String,
    crypto_chain: String,
    crypto_amount: Decimal,
    exchange_rate: Option<Decimal>,
    checkout_url: Option<String>,
    wallet_address: Option<String>,
    idempotency_key: String,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<FiatOrderRow> for FiatConversionOrder {
    fn from(row: FiatOrderRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            invoice_id: row.invoice_id,
            payment_id: row.payment_id,
            provider: parse_provider(&row.provider),
            external_order_id: row.external_order_id,
            status: parse_status(&row.status),
            fiat_currency: row.fiat_currency,
            fiat_amount: row.fiat_amount,
            crypto_asset: row.crypto_asset,
            crypto_chain: row.crypto_chain,
            crypto_amount: row.crypto_amount,
            exchange_rate: row.exchange_rate,
            checkout_url: row.checkout_url,
            wallet_address: row.wallet_address,
            idempotency_key: row.idempotency_key,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn parse_provider(value: &str) -> FiatProvider {
    FiatProvider::parse(value).unwrap_or(FiatProvider::Transak)
}

fn parse_status(value: &str) -> FiatOrderStatus {
    match value {
        "processing" => FiatOrderStatus::Processing,
        "completed" => FiatOrderStatus::Completed,
        "failed" => FiatOrderStatus::Failed,
        "cancelled" => FiatOrderStatus::Cancelled,
        _ => FiatOrderStatus::Pending,
    }
}
