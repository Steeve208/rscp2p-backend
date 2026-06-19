use std::sync::Arc;

use reqwest::Client;
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use crate::internal::config::Config;
use crate::internal::core::financial_gateway::crypto_repository::{
    CreateCryptoOrderParams, CryptoOrderRepository, CryptoOrderRow, CryptoOrderType,
};
use crate::internal::core::financial_gateway::error::{GatewayError, GatewayResult};
use crate::internal::core::financial_gateway::models::{
    CryptoOrderResponse, CryptoQuoteRequest, CryptoQuoteResponse, CryptoTradeRequest,
    CryptoWidgetResponse,
};
use crate::internal::providers::striga::repository::StrigaRepository;
use crate::internal::providers::transak::api::TransakApiClient;
use crate::internal::providers::transak::config::TransakProviderConfig;
use crate::internal::providers::transak::models::OrderStatus;
use crate::internal::providers::transak::onramp::OnRampService;
use crate::internal::providers::transak::webhooks::{parse_webhook, verify_webhook_secret};
use crate::internal::providers::transak::widget::{build_widget_url, WidgetFlow};

#[derive(Clone)]
pub struct CryptoService {
    config: Arc<Config>,
    transak_config: TransakProviderConfig,
    transak_client: TransakApiClient,
    onramp: OnRampService,
    orders: CryptoOrderRepository,
    audit: StrigaRepository,
}

impl CryptoService {
    pub fn new(
        http: Client,
        config: Arc<Config>,
        orders: CryptoOrderRepository,
        audit: StrigaRepository,
    ) -> Self {
        let transak_config = config.providers.transak.clone();
        let transak_client = TransakApiClient::new(http.clone(), transak_config.clone());
        let onramp = OnRampService::new(http, transak_config.clone());

        Self {
            config,
            transak_config,
            transak_client,
            onramp,
            orders,
            audit,
        }
    }

    pub async fn get_quote(&self, req: CryptoQuoteRequest) -> GatewayResult<CryptoQuoteResponse> {
        let chain = req.crypto_chain.clone().unwrap_or_else(|| "ethereum".into());
        let side = req.side.as_deref().unwrap_or("BUY").to_uppercase();

        if self.onramp.uses_mock() {
            return mock_quote(&req, &side);
        }

        let network = TransakApiClient::map_chain(&chain);
        let quote = if side == "SELL" {
            let crypto_amount = req
                .crypto_amount
                .filter(|a| *a > Decimal::ZERO)
                .ok_or_else(|| GatewayError::Validation("crypto_amount required for sell".into()))?;
            self.transak_client
                .price_quote_sell(&req.fiat_currency, &req.crypto_asset, &network, crypto_amount)
                .await?
        } else {
            let fiat_amount = req
                .fiat_amount
                .filter(|a| *a > Decimal::ZERO)
                .ok_or_else(|| GatewayError::Validation("fiat_amount required for buy".into()))?;
            self.transak_client
                .price_quote_buy(&req.fiat_currency, &req.crypto_asset, &network, fiat_amount)
                .await?
        };

        Ok(CryptoQuoteResponse {
            fiat_currency: quote.fiat_currency,
            fiat_amount: quote.fiat_amount,
            crypto_asset: quote.crypto_asset,
            crypto_amount: quote.crypto_amount,
            rate: quote.exchange_rate,
            side,
        })
    }

    pub async fn buy_crypto(
        &self,
        user_id: Uuid,
        req: CryptoTradeRequest,
    ) -> GatewayResult<CryptoWidgetResponse> {
        self.start_trade(user_id, req, CryptoOrderType::Buy, WidgetFlow::Buy)
            .await
    }

    pub async fn sell_crypto(
        &self,
        user_id: Uuid,
        req: CryptoTradeRequest,
    ) -> GatewayResult<CryptoWidgetResponse> {
        self.start_trade(user_id, req, CryptoOrderType::Sell, WidgetFlow::Sell)
            .await
    }

    pub async fn off_ramp(
        &self,
        user_id: Uuid,
        req: CryptoTradeRequest,
    ) -> GatewayResult<CryptoWidgetResponse> {
        self.start_trade(user_id, req, CryptoOrderType::OffRamp, WidgetFlow::Sell)
            .await
    }

    async fn start_trade(
        &self,
        user_id: Uuid,
        req: CryptoTradeRequest,
        order_type: CryptoOrderType,
        flow: WidgetFlow,
    ) -> GatewayResult<CryptoWidgetResponse> {
        let chain = req.crypto_chain.unwrap_or_else(|| "ethereum".into());

        let order = self
            .orders
            .create_order(&CreateCryptoOrderParams {
                user_id,
                order_type,
                fiat_currency: Some(req.fiat_currency.clone()),
                fiat_amount: req.fiat_amount,
                crypto_asset: Some(req.crypto_asset.clone()),
                crypto_amount: req.crypto_amount,
                metadata: json!({ "chain": chain, "wallet_address": req.wallet_address }),
            })
            .await?;

        let widget_url = build_widget_url(
            &self.transak_config,
            flow,
            &req.fiat_currency,
            &req.crypto_asset,
            &chain,
            req.fiat_amount,
            req.crypto_amount,
            req.wallet_address.as_deref(),
            order.id,
            user_id,
        );

        self.orders
            .update_status(
                order.id,
                "PENDING",
                None,
                Some(&json!({ "widget_url": widget_url })),
            )
            .await?;

        self.audit
            .update_provider_status("transak", "healthy", None)
            .await?;

        Ok(CryptoWidgetResponse {
            order_id: order.id,
            widget_url,
            status: "PENDING".into(),
        })
    }

    pub async fn get_order(&self, user_id: Uuid, order_id: Uuid) -> GatewayResult<CryptoOrderResponse> {
        let row = self
            .orders
            .get_order(user_id, order_id)
            .await?
            .ok_or(GatewayError::NotFound)?;
        Ok(row.into())
    }

    pub async fn list_orders(&self, user_id: Uuid) -> GatewayResult<Vec<CryptoOrderResponse>> {
        let rows = self.orders.list_user_orders(user_id, 50).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_supported_assets(&self) -> GatewayResult<Vec<String>> {
        Ok(vec![
            "BTC".into(),
            "ETH".into(),
            "USDC".into(),
            "USDT".into(),
            "RSC".into(),
        ])
    }

    pub async fn handle_transak_webhook(
        &self,
        secret: Option<&str>,
        body: serde_json::Value,
    ) -> GatewayResult<()> {
        verify_webhook_secret(&self.transak_config, self.config.environment, secret)?;

        let event = parse_webhook(&body)?;
        let event_type = map_event_type(&event.status);

        let log_id = self
            .audit
            .log_webhook(
                "transak",
                &event_type,
                Some(&event.order_id),
                &body,
                false,
                None,
            )
            .await?;

        let result = self.process_webhook(&event, &event_type).await;

        match &result {
            Ok(()) => {
                self.audit.mark_webhook_processed(log_id, true, None).await?;
            }
            Err(e) => {
                self.audit
                    .mark_webhook_processed(log_id, false, Some(&e.to_string()))
                    .await?;
            }
        }

        result
    }

    async fn process_webhook(
        &self,
        event: &crate::internal::providers::transak::webhooks::OrderNotification,
        event_type: &str,
    ) -> GatewayResult<()> {
        let status = map_order_status(&event.status);

        let order = if let Some(ref partner_id) = event.partner_order_id {
            if let Ok(uuid) = Uuid::parse_str(partner_id) {
                self.orders.find_by_partner_order_id(uuid).await?
            } else {
                None
            }
        } else {
            None
        }
        .or(self.orders.find_by_external_id(&event.order_id).await?);

        if let Some(order) = order {
            self.orders
                .update_status(
                    order.id,
                    status,
                    Some(&event.order_id),
                    Some(&json!({ "last_event": event_type })),
                )
                .await?;
        }

        let _ = self
            .audit
            .update_provider_status("transak", "healthy", None)
            .await;

        Ok(())
    }

    pub async fn sync_pending_orders(&self) -> GatewayResult<u32> {
        let pending = self.orders.list_pending_sync(100).await?;
        let mut synced = 0u32;

        for order in pending {
            let Some(ref ext_id) = order.external_order_id else {
                continue;
            };
            if self.onramp.uses_mock() {
                continue;
            }

            match self.transak_client.get_order(ext_id).await {
                Ok(remote) => {
                    let status = map_order_status(&remote.status);
                    let _ = self
                        .orders
                        .update_status(order.id, status, None, None)
                        .await;
                    synced += 1;
                }
                Err(e) => {
                    tracing::warn!(order_id = %order.id, error = %e, "crypto order sync failed");
                    let _ = self
                        .audit
                        .update_provider_status("transak", "degraded", Some(&e.to_string()))
                        .await;
                }
            }
        }

        let _ = self
            .audit
            .update_provider_status("transak", "healthy", None)
            .await;

        Ok(synced)
    }
}

fn mock_quote(req: &CryptoQuoteRequest, side: &str) -> GatewayResult<CryptoQuoteResponse> {
    let fiat = req.fiat_amount.unwrap_or(Decimal::new(100, 0));
    let crypto = req
        .crypto_amount
        .unwrap_or(fiat / Decimal::new(96, 2));
    Ok(CryptoQuoteResponse {
        fiat_currency: req.fiat_currency.to_uppercase(),
        fiat_amount: fiat,
        crypto_asset: req.crypto_asset.to_uppercase(),
        crypto_amount: crypto,
        rate: if crypto > Decimal::ZERO {
            fiat / crypto
        } else {
            Decimal::ONE
        },
        side: side.to_string(),
    })
}

fn map_order_status(status: &OrderStatus) -> &'static str {
    match status {
        OrderStatus::Completed => "COMPLETED",
        OrderStatus::Failed => "FAILED",
        OrderStatus::Cancelled => "CANCELLED",
        OrderStatus::Processing | OrderStatus::AwaitingPayment => "PROCESSING",
        OrderStatus::Expired => "FAILED",
        OrderStatus::Unknown => "PENDING",
    }
}

fn map_event_type(status: &OrderStatus) -> String {
    match status {
        OrderStatus::Completed => "ORDER_COMPLETED".into(),
        OrderStatus::Failed => "ORDER_FAILED".into(),
        OrderStatus::Cancelled => "ORDER_CANCELLED".into(),
        OrderStatus::Processing | OrderStatus::AwaitingPayment => "ORDER_PENDING".into(),
        OrderStatus::Unknown | OrderStatus::Expired => "ORDER_CREATED".into(),
    }
}

impl From<CryptoOrderRow> for CryptoOrderResponse {
    fn from(row: CryptoOrderRow) -> Self {
        Self {
            id: row.id,
            order_type: row.order_type,
            status: row.status,
            fiat_currency: row.fiat_currency,
            fiat_amount: row.fiat_amount,
            crypto_asset: row.crypto_asset,
            crypto_amount: row.crypto_amount,
            widget_url: row
                .metadata
                .get("widget_url")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<crate::internal::providers::transak::error::TransakError> for GatewayError {
    fn from(err: crate::internal::providers::transak::error::TransakError) -> Self {
        match err {
            crate::internal::providers::transak::error::TransakError::NotConfigured => {
                Self::NotConfigured
            }
            crate::internal::providers::transak::error::TransakError::Validation(m) => {
                Self::Validation(m)
            }
            crate::internal::providers::transak::error::TransakError::Upstream(m) => Self::Upstream(m),
            crate::internal::providers::transak::error::TransakError::WebhookForbidden => {
                Self::Forbidden
            }
            crate::internal::providers::transak::error::TransakError::Parse(m) => Self::Upstream(m),
        }
    }
}
