use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::core::financial_gateway::error::GatewayResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoOrderType {
    Buy,
    Sell,
    OffRamp,
}

impl CryptoOrderType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
            Self::OffRamp => "OFF_RAMP",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateCryptoOrderParams {
    pub user_id: Uuid,
    pub order_type: CryptoOrderType,
    pub fiat_currency: Option<String>,
    pub fiat_amount: Option<Decimal>,
    pub crypto_asset: Option<String>,
    pub crypto_amount: Option<Decimal>,
    pub metadata: Value,
}

#[derive(Clone)]
pub struct CryptoOrderRepository {
    pool: PgPool,
}

impl CryptoOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_order(&self, params: &CreateCryptoOrderParams) -> GatewayResult<CryptoOrderRow> {
        let row = sqlx::query_as::<_, CryptoOrderRow>(
            r#"
            INSERT INTO crypto_orders (
                user_id, provider, order_type, status,
                fiat_currency, fiat_amount, crypto_asset, crypto_amount, metadata
            )
            VALUES ($1, 'transak', $2, 'PENDING', $3, $4, $5, $6, $7)
            RETURNING id, user_id, provider, external_order_id, order_type, status,
                      fiat_currency, fiat_amount, crypto_asset, crypto_amount,
                      metadata, created_at, updated_at
            "#,
        )
        .bind(params.user_id)
        .bind(params.order_type.as_str())
        .bind(&params.fiat_currency)
        .bind(params.fiat_amount)
        .bind(&params.crypto_asset)
        .bind(params.crypto_amount)
        .bind(&params.metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_order(&self, user_id: Uuid, order_id: Uuid) -> GatewayResult<Option<CryptoOrderRow>> {
        let row = sqlx::query_as::<_, CryptoOrderRow>(
            r#"
            SELECT id, user_id, provider, external_order_id, order_type, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_amount,
                   metadata, created_at, updated_at
            FROM crypto_orders WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_user_orders(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> GatewayResult<Vec<CryptoOrderRow>> {
        let rows = sqlx::query_as::<_, CryptoOrderRow>(
            r#"
            SELECT id, user_id, provider, external_order_id, order_type, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_amount,
                   metadata, created_at, updated_at
            FROM crypto_orders
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn find_by_external_id(&self, external_id: &str) -> GatewayResult<Option<CryptoOrderRow>> {
        let row = sqlx::query_as::<_, CryptoOrderRow>(
            r#"
            SELECT id, user_id, provider, external_order_id, order_type, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_amount,
                   metadata, created_at, updated_at
            FROM crypto_orders WHERE external_order_id = $1
            "#,
        )
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn find_by_partner_order_id(&self, partner_order_id: Uuid) -> GatewayResult<Option<CryptoOrderRow>> {
        let row = sqlx::query_as::<_, CryptoOrderRow>(
            r#"
            SELECT id, user_id, provider, external_order_id, order_type, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_amount,
                   metadata, created_at, updated_at
            FROM crypto_orders WHERE id = $1
            "#,
        )
        .bind(partner_order_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_status(
        &self,
        order_id: Uuid,
        status: &str,
        external_order_id: Option<&str>,
        metadata_patch: Option<&Value>,
    ) -> GatewayResult<()> {
        if let Some(patch) = metadata_patch {
            sqlx::query(
                r#"
                UPDATE crypto_orders SET
                    status = $2,
                    external_order_id = COALESCE($3, external_order_id),
                    metadata = metadata || $4,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(order_id)
            .bind(status)
            .bind(external_order_id)
            .bind(patch)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE crypto_orders SET
                    status = $2,
                    external_order_id = COALESCE($3, external_order_id),
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(order_id)
            .bind(status)
            .bind(external_order_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn list_pending_sync(&self, limit: i64) -> GatewayResult<Vec<CryptoOrderRow>> {
        let rows = sqlx::query_as::<_, CryptoOrderRow>(
            r#"
            SELECT id, user_id, provider, external_order_id, order_type, status,
                   fiat_currency, fiat_amount, crypto_asset, crypto_amount,
                   metadata, created_at, updated_at
            FROM crypto_orders
            WHERE status IN ('PENDING', 'PROCESSING')
              AND external_order_id IS NOT NULL
            ORDER BY updated_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct CryptoOrderRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub external_order_id: Option<String>,
    pub order_type: String,
    pub status: String,
    pub fiat_currency: Option<String>,
    pub fiat_amount: Option<Decimal>,
    pub crypto_asset: Option<String>,
    pub crypto_amount: Option<Decimal>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
