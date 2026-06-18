use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::providers::striga::error::StrigaResult;
use crate::internal::providers::striga::models::{CardStatus, CardType, KycStatus};

#[derive(Clone)]
pub struct StrigaRepository {
    pool: PgPool,
}

impl StrigaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn set_striga_user_id(
        &self,
        user_id: Uuid,
        striga_user_id: &str,
    ) -> StrigaResult<()> {
        sqlx::query(
            r#"
            UPDATE users SET striga_user_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(striga_user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_striga_user_id(&self, user_id: Uuid) -> StrigaResult<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT striga_user_id FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.and_then(|(id,)| id))
    }

    pub async fn find_user_by_striga_id(&self, striga_user_id: &str) -> StrigaResult<Option<Uuid>> {
        let row: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM users WHERE striga_user_id = $1")
                .bind(striga_user_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn upsert_kyc_record(
        &self,
        user_id: Uuid,
        striga_user_id: &str,
        status: KycStatus,
        tier: i16,
        token: Option<&str>,
        rejection_reason: Option<&str>,
    ) -> StrigaResult<()> {
        sqlx::query(
            r#"
            INSERT INTO kyc_records (user_id, striga_user_id, status, tier, verification_token, rejection_reason)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id) DO UPDATE SET
                status = EXCLUDED.status,
                tier = EXCLUDED.tier,
                verification_token = COALESCE(EXCLUDED.verification_token, kyc_records.verification_token),
                rejection_reason = EXCLUDED.rejection_reason,
                updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(striga_user_id)
        .bind(status.as_str())
        .bind(tier)
        .bind(token)
        .bind(rejection_reason)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_kyc_record(&self, user_id: Uuid) -> StrigaResult<Option<KycRecordRow>> {
        let row = sqlx::query_as::<_, KycRecordRow>(
            r#"
            SELECT id, user_id, status, tier, striga_user_id, verification_token,
                   rejection_reason, created_at, updated_at
            FROM kyc_records WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn insert_card(
        &self,
        user_id: Uuid,
        striga_card_id: &str,
        card_type: CardType,
        status: CardStatus,
        last_four: Option<&str>,
        expiry_month: Option<i16>,
        expiry_year: Option<i16>,
        metadata: &Value,
    ) -> StrigaResult<Uuid> {
        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO cards (user_id, striga_card_id, card_type, card_status, last_four,
                               expiry_month, expiry_year, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(striga_card_id)
        .bind(card_type.as_str())
        .bind(status.as_str())
        .bind(last_four)
        .bind(expiry_month)
        .bind(expiry_year)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn update_card_status(
        &self,
        striga_card_id: &str,
        status: CardStatus,
    ) -> StrigaResult<()> {
        sqlx::query(
            r#"
            UPDATE cards SET card_status = $2, updated_at = NOW()
            WHERE striga_card_id = $1
            "#,
        )
        .bind(striga_card_id)
        .bind(status.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_card_by_striga_id(
        &self,
        striga_card_id: &str,
    ) -> StrigaResult<Option<CardRow>> {
        let row = sqlx::query_as::<_, CardRow>(
            r#"
            SELECT id, user_id, striga_card_id, card_type, card_status, last_four,
                   expiry_month, expiry_year, created_at, updated_at
            FROM cards WHERE striga_card_id = $1
            "#,
        )
        .bind(striga_card_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_user_cards(&self, user_id: Uuid) -> StrigaResult<Vec<CardRow>> {
        let rows = sqlx::query_as::<_, CardRow>(
            r#"
            SELECT id, user_id, striga_card_id, card_type, card_status, last_four,
                   expiry_month, expiry_year, created_at, updated_at
            FROM cards WHERE user_id = $1 ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn upsert_card_transaction(
        &self,
        card_id: Uuid,
        external_id: &str,
        amount: Decimal,
        currency: &str,
        direction: &str,
        merchant_name: Option<&str>,
        status: &str,
        transacted_at: DateTime<Utc>,
        metadata: &Value,
    ) -> StrigaResult<()> {
        sqlx::query(
            r#"
            INSERT INTO card_transactions (card_id, external_id, amount, currency, direction,
                                           merchant_name, status, transacted_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (card_id, external_id) DO UPDATE SET
                amount = EXCLUDED.amount,
                status = EXCLUDED.status,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(card_id)
        .bind(external_id)
        .bind(amount)
        .bind(currency)
        .bind(direction)
        .bind(merchant_name)
        .bind(status)
        .bind(transacted_at)
        .bind(metadata)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn log_webhook(
        &self,
        provider: &str,
        event_type: &str,
        external_id: Option<&str>,
        payload: &Value,
        processed: bool,
        error_message: Option<&str>,
    ) -> StrigaResult<Uuid> {
        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO webhook_logs (provider, event_type, external_id, payload, processed, error_message, processed_at)
            VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $5 THEN NOW() ELSE NULL END)
            RETURNING id
            "#,
        )
        .bind(provider)
        .bind(event_type)
        .bind(external_id)
        .bind(payload)
        .bind(processed)
        .bind(error_message)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn update_provider_status(
        &self,
        provider: &str,
        status: &str,
        last_error: Option<&str>,
    ) -> StrigaResult<()> {
        sqlx::query(
            r#"
            INSERT INTO provider_status (provider, status, last_sync_at, last_error, updated_at)
            VALUES ($1, $2, NOW(), $3, NOW())
            ON CONFLICT (provider) DO UPDATE SET
                status = EXCLUDED.status,
                last_sync_at = EXCLUDED.last_sync_at,
                last_error = EXCLUDED.last_error,
                updated_at = NOW()
            "#,
        )
        .bind(provider)
        .bind(status)
        .bind(last_error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_recent_webhooks(
        &self,
        provider: Option<&str>,
        limit: i64,
    ) -> StrigaResult<Vec<WebhookLogRow>> {
        let rows = if let Some(p) = provider {
            sqlx::query_as::<_, WebhookLogRow>(
                r#"
                SELECT id, provider, event_type, external_id, processed, error_message, received_at
                FROM webhook_logs WHERE provider = $1
                ORDER BY received_at DESC LIMIT $2
                "#,
            )
            .bind(p)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, WebhookLogRow>(
                r#"
                SELECT id, provider, event_type, external_id, processed, error_message, received_at
                FROM webhook_logs ORDER BY received_at DESC LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    pub async fn get_provider_status(&self) -> StrigaResult<Vec<ProviderStatusRow>> {
        let rows = sqlx::query_as::<_, ProviderStatusRow>(
            r#"
            SELECT provider, status, last_sync_at, last_error, updated_at
            FROM provider_status ORDER BY provider
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_cards_for_sync(&self) -> StrigaResult<Vec<CardRow>> {
        let rows = sqlx::query_as::<_, CardRow>(
            r#"
            SELECT id, user_id, striga_card_id, card_type, card_status, last_four,
                   expiry_month, expiry_year, created_at, updated_at
            FROM cards
            WHERE card_status NOT IN ('TERMINATED')
            ORDER BY updated_at ASC
            LIMIT 500
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn mark_webhook_processed(
        &self,
        log_id: Uuid,
        processed: bool,
        error_message: Option<&str>,
    ) -> StrigaResult<()> {
        sqlx::query(
            r#"
            UPDATE webhook_logs
            SET processed = $2, error_message = $3, processed_at = CASE WHEN $2 THEN NOW() ELSE processed_at END
            WHERE id = $1
            "#,
        )
        .bind(log_id)
        .bind(processed)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct KycRecordRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub tier: i16,
    pub striga_user_id: Option<String>,
    pub verification_token: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct CardRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub striga_card_id: String,
    pub card_type: String,
    pub card_status: String,
    pub last_four: Option<String>,
    pub expiry_month: Option<i16>,
    pub expiry_year: Option<i16>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct WebhookLogRow {
    pub id: Uuid,
    pub provider: String,
    pub event_type: String,
    pub external_id: Option<String>,
    pub processed: bool,
    pub error_message: Option<String>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct ProviderStatusRow {
    pub provider: String,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}
