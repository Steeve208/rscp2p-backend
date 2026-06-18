//! Swap engine — pricing, routing, fees, provider-agnostic execution.

use std::sync::Arc;

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::internal::swaps::config::SwapsConfig;
use crate::internal::swaps::error::{SwapError, SwapResult};
use crate::internal::swaps::models::{
    CreateSwapOrderRequest, CreateSwapOrderResponse, SwapOrder, SwapPair, SwapProviderInfo,
    SwapQuoteRequest, SwapQuoteResponse, supported_pairs,
};
use crate::internal::swaps::pricing::PricingEngine;
use crate::internal::swaps::providers::factory::build_registry;
use crate::internal::swaps::registry::SwapProviderRegistry;
use crate::internal::swaps::repository::SwapRepository;
use crate::internal::swaps::traits::ExecuteRequest;

pub struct SwapService {
    repo: SwapRepository,
    registry: SwapProviderRegistry,
    config: SwapsConfig,
}

impl SwapService {
    pub fn new(pool: PgPool, config: SwapsConfig, http: reqwest::Client) -> Self {
        let registry = build_registry(&config, http);
        Self {
            repo: SwapRepository::new(pool),
            registry,
            config,
        }
    }

    pub fn list_pairs(&self) -> Vec<SwapPair> {
        supported_pairs()
    }

    pub fn list_providers(&self) -> Vec<SwapProviderInfo> {
        self.registry.list()
    }

    pub async fn quote(&self, request: SwapQuoteRequest) -> SwapResult<SwapQuoteResponse> {
        request
            .validate()
            .map_err(|e| SwapError::Validation(e.to_string()))?;

        if self.registry.is_empty() {
            return Err(SwapError::ProviderUnavailable);
        }

        PricingEngine::quote(&self.registry, &self.config, request).await
    }

    pub async fn create_order(
        &self,
        user_id: Uuid,
        request: CreateSwapOrderRequest,
    ) -> SwapResult<CreateSwapOrderResponse> {
        request
            .validate()
            .map_err(|e| SwapError::Validation(e.to_string()))?;

        if request.from_amount <= Decimal::ZERO {
            return Err(SwapError::InvalidAmount);
        }

        if let Some(existing) = self.repo.find_by_idempotency(&request.idempotency_key).await? {
            let quote = ProviderQuoteView {
                provider_id: existing.provider_id.clone(),
                venue_kind: existing.venue_kind,
                from_asset: existing.from_asset.clone(),
                to_asset: existing.to_asset.clone(),
                from_amount: existing.from_amount,
                to_amount: existing.to_amount,
                exchange_rate: existing.exchange_rate.unwrap_or(Decimal::ONE),
                fee_provider: existing.fee_provider,
                fee_network: existing.fee_network,
                mock: false,
            };
            return Ok(CreateSwapOrderResponse {
                order: existing,
                quote,
                idempotent_replay: true,
            });
        }

        let quote = self
            .quote(SwapQuoteRequest {
                from_asset: request.from_asset.clone(),
                to_asset: request.to_asset.clone(),
                from_chain: request.from_chain.clone(),
                to_chain: request.to_chain.clone(),
                from_amount: Some(request.from_amount),
                to_amount: None,
                provider_id: request.provider_id.clone(),
                slippage_bps: request.slippage_bps,
            })
            .await?;

        let provider = self
            .registry
            .get(&quote.best.provider_id)
            .ok_or(SwapError::NoRoute)?;

        let slippage_factor =
            Decimal::ONE - Decimal::from(request.slippage_bps) / Decimal::from(10_000);
        let min_to = quote.best.to_amount * slippage_factor;

        let pair = quote.pair.clone();
        let exec = provider
            .execute(&ExecuteRequest {
                pair: pair.clone(),
                from_amount: request.from_amount,
                min_to_amount: min_to,
                slippage_bps: request.slippage_bps,
                idempotency_key: request.idempotency_key.clone(),
            })
            .await?;

        let order = self
            .repo
            .create_order(
                user_id,
                &quote.best.provider_id,
                quote.best.venue_kind,
                &pair.from_asset,
                &pair.to_asset,
                pair.from_chain.as_deref(),
                pair.to_chain.as_deref(),
                exec.from_amount,
                exec.to_amount,
                quote.fees.platform_amount,
                quote.fees.provider_amount,
                quote.fees.network_amount,
                quote.best.exchange_rate,
                request.slippage_bps,
                &request.idempotency_key,
                &exec.external_order_id,
                &exec.raw,
            )
            .await?;

        Ok(CreateSwapOrderResponse {
            order,
            quote: quote.best,
            idempotent_replay: false,
        })
    }

    pub async fn get_order(&self, user_id: Uuid, order_id: Uuid) -> SwapResult<SwapOrder> {
        let order = self
            .repo
            .find_by_id(order_id)
            .await?
            .ok_or(SwapError::OrderNotFound)?;

        if order.user_id != user_id {
            return Err(SwapError::Forbidden);
        }

        Ok(order)
    }
}

// Re-export for idempotent stub
use crate::internal::swaps::models::ProviderQuoteView;

#[derive(Clone)]
pub struct SwapServiceHandle(pub Arc<SwapService>);

impl SwapServiceHandle {
    pub fn new(pool: PgPool, config: SwapsConfig, http: reqwest::Client) -> Self {
        Self(Arc::new(SwapService::new(pool, config, http)))
    }
}

impl std::ops::Deref for SwapServiceHandle {
    type Target = SwapService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
