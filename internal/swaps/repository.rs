use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::swaps::error::{SwapError, SwapResult};
use crate::internal::swaps::models::{LiquidityVenueKind, SwapOrder, SwapOrderStatus};
use crate::internal::swaps::traits::ExecuteResult;

#[derive(Clone)]
pub struct SwapRepository {
    pool: PgPool,
}

impl SwapRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: Uuid) -> SwapResult<Option<SwapOrder>> {
        let row = sqlx::query_as::<_, SwapOrderRow>(
            r#"
            SELECT id, user_id, provider_id, venue_kind, from_asset, to_asset,
                   from_chain, to_chain, from_amount, to_amount,
                   fee_platform, fee_provider, fee_network, exchange_rate,
                   slippage_bps, status, idempotency_key, external_order_id,
                   created_at, updated_at, completed_at
            FROM swap_orders WHERE id = $1
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
    ) -> SwapResult<Option<SwapOrder>> {
        let row = sqlx::query_as::<_, SwapOrderRow>(
            r#"
            SELECT id, user_id, provider_id, venue_kind, from_asset, to_asset,
                   from_chain, to_chain, from_amount, to_amount,
                   fee_platform, fee_provider, fee_network, exchange_rate,
                   slippage_bps, status, idempotency_key, external_order_id,
                   created_at, updated_at, completed_at
            FROM swap_orders WHERE idempotency_key = $1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn create_order(
        &self,
        user_id: Uuid,
        provider_id: &str,
        venue_kind: LiquidityVenueKind,
        from_asset: &str,
        to_asset: &str,
        from_chain: Option<&str>,
        to_chain: Option<&str>,
        from_amount: Decimal,
        to_amount: Decimal,
        fee_platform: Decimal,
        fee_provider: Decimal,
        fee_network: Decimal,
        exchange_rate: Decimal,
        slippage_bps: u32,
        idempotency_key: &str,
        external_order_id: &str,
        route_snapshot: &serde_json::Value,
    ) -> SwapResult<SwapOrder> {
        let row = sqlx::query_as::<_, SwapOrderRow>(
            r#"
            INSERT INTO swap_orders (
                user_id, provider_id, venue_kind, from_asset, to_asset,
                from_chain, to_chain, from_amount, to_amount,
                fee_platform, fee_provider, fee_network, exchange_rate,
                slippage_bps, status, idempotency_key, external_order_id,
                route_snapshot, completed_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, 'completed', $15, $16, $17, NOW()
            )
            RETURNING id, user_id, provider_id, venue_kind, from_asset, to_asset,
                      from_chain, to_chain, from_amount, to_amount,
                      fee_platform, fee_provider, fee_network, exchange_rate,
                      slippage_bps, status, idempotency_key, external_order_id,
                      created_at, updated_at, completed_at
            "#,
        )
        .bind(user_id)
        .bind(provider_id)
        .bind(venue_kind.as_str())
        .bind(from_asset)
        .bind(to_asset)
        .bind(from_chain)
        .bind(to_chain)
        .bind(from_amount)
        .bind(to_amount)
        .bind(fee_platform)
        .bind(fee_provider)
        .bind(fee_network)
        .bind(exchange_rate)
        .bind(slippage_bps as i32)
        .bind(idempotency_key)
        .bind(external_order_id)
        .bind(route_snapshot)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db) = &e {
                if db.constraint() == Some("unique_swap_order_idempotency") {
                    return SwapError::Conflict("swap idempotency key already used".into());
                }
            }
            SwapError::Database(e)
        })?;

        Ok(row.into())
    }
}

#[derive(sqlx::FromRow)]
struct SwapOrderRow {
    id: Uuid,
    user_id: Uuid,
    provider_id: String,
    venue_kind: String,
    from_asset: String,
    to_asset: String,
    from_chain: Option<String>,
    to_chain: Option<String>,
    from_amount: Decimal,
    to_amount: Decimal,
    fee_platform: Decimal,
    fee_provider: Decimal,
    fee_network: Decimal,
    exchange_rate: Option<Decimal>,
    slippage_bps: i32,
    status: String,
    idempotency_key: String,
    external_order_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<SwapOrderRow> for SwapOrder {
    fn from(row: SwapOrderRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            provider_id: row.provider_id,
            venue_kind: parse_venue(&row.venue_kind),
            from_asset: row.from_asset,
            to_asset: row.to_asset,
            from_chain: row.from_chain,
            to_chain: row.to_chain,
            from_amount: row.from_amount,
            to_amount: row.to_amount,
            fee_platform: row.fee_platform,
            fee_provider: row.fee_provider,
            fee_network: row.fee_network,
            exchange_rate: row.exchange_rate,
            slippage_bps: row.slippage_bps as u32,
            status: parse_status(&row.status),
            idempotency_key: row.idempotency_key,
            external_order_id: row.external_order_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
        }
    }
}

fn parse_venue(value: &str) -> LiquidityVenueKind {
    LiquidityVenueKind::parse(value).unwrap_or(LiquidityVenueKind::Dex)
}

fn parse_status(value: &str) -> SwapOrderStatus {
    match value {
        "processing" => SwapOrderStatus::Processing,
        "completed" => SwapOrderStatus::Completed,
        "failed" => SwapOrderStatus::Failed,
        "cancelled" => SwapOrderStatus::Cancelled,
        _ => SwapOrderStatus::Pending,
    }
}

#[allow(dead_code)]
impl SwapRepository {
    pub fn snapshot_execute(exec: &ExecuteResult, quote_raw: &serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "execute": exec,
            "quote": quote_raw,
        })
    }
}
