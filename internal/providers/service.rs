//! Fiat conversion orchestration — quotes, orders, webhooks, invoice settlement.

use std::sync::Arc;

use reqwest::Client;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::internal::config::Config;
use crate::internal::payments::error::PaymentError;
use crate::internal::payments::models::{InvoiceStatus, PaymentMethod};
use crate::internal::payments::repository::PaymentRepository;
use crate::internal::providers::error::{ProviderError, ProviderResult};
use crate::internal::providers::models::{
    FiatConversionOrder, FiatOrderStatus, FiatProvider, FiatProviderInfo, FiatQuoteRequest,
    FiatQuoteResponse, ProviderQuote, StartFiatInvoicePayRequest, StartFiatInvoicePayResponse,
};
use crate::internal::providers::repository::FiatConversionRepository;
use crate::internal::providers::traits::FiatOnRampProvider;
use crate::internal::providers::transak::{self, TransakProvider};
use crate::internal::wallets::models::RecordDepositRequest;
use crate::internal::wallets::services::WalletServiceHandle;

pub struct FiatConversionService {
    repo: FiatConversionRepository,
    payments: PaymentRepository,
    wallets: WalletServiceHandle,
    clearing_wallet_id: Uuid,
    transak: TransakProvider,
    config: Arc<Config>,
}

impl FiatConversionService {
    pub fn new(
        pool: PgPool,
        http: Client,
        config: Arc<Config>,
        wallets: WalletServiceHandle,
        clearing_wallet_id: Uuid,
    ) -> Self {
        let providers = &config.providers;
        Self {
            repo: FiatConversionRepository::new(pool.clone()),
            payments: PaymentRepository::new(pool),
            wallets,
            clearing_wallet_id,
            transak: TransakProvider::new(http, providers.transak.clone()),
            config,
        }
    }

    pub fn list_providers(&self) -> Vec<FiatProviderInfo> {
        vec![FiatProviderInfo {
            provider: FiatProvider::Transak,
            configured: self.transak.is_configured(),
            mock_mode: self.transak.uses_mock(),
        }]
    }

    pub async fn quote(&self, req: FiatQuoteRequest) -> ProviderResult<Vec<FiatQuoteResponse>> {
        req.validate()
            .map_err(|e| ProviderError::Validation(e.to_string()))?;

        let asset = normalize_token(&req.crypto_asset)?;
        let chain = normalize_chain(&req.crypto_chain)?;

        let providers: Vec<&dyn FiatOnRampProvider> = match req
            .provider
            .as_deref()
            .and_then(FiatProvider::parse)
        {
            Some(FiatProvider::Transak) => vec![&self.transak],
            None => vec![&self.transak],
        };

        let mut out = Vec::with_capacity(providers.len());
        for provider in providers {
            let quote = provider
                .quote(
                    &req.fiat_currency,
                    req.fiat_amount,
                    &asset,
                    &chain,
                    req.crypto_amount,
                    None,
                )
                .await?;
            out.push(to_quote_response(provider.provider(), quote));
        }
        Ok(out)
    }

    pub async fn get_order(&self, user_id: Uuid, order_id: Uuid) -> ProviderResult<FiatConversionOrder> {
        let order = self
            .repo
            .find_by_id(order_id)
            .await?
            .ok_or(ProviderError::OrderNotFound)?;
        if order.user_id != user_id {
            return Err(ProviderError::Forbidden);
        }
        Ok(order)
    }

    pub async fn start_invoice_fiat_pay(
        &self,
        user_id: Uuid,
        invoice_id: Uuid,
        req: StartFiatInvoicePayRequest,
    ) -> ProviderResult<StartFiatInvoicePayResponse> {
        req.validate()
            .map_err(|e| ProviderError::Validation(e.to_string()))?;

        if let Some(existing) = self.repo.find_by_idempotency(&req.idempotency_key).await? {
            let quote = FiatQuoteResponse {
                provider: existing.provider,
                fiat_currency: existing.fiat_currency.clone(),
                fiat_amount: existing.fiat_amount,
                crypto_asset: existing.crypto_asset.clone(),
                crypto_chain: existing.crypto_chain.clone(),
                crypto_amount: existing.crypto_amount,
                exchange_rate: existing.exchange_rate.unwrap_or(Decimal::ONE),
                fee_fiat: None,
                mock: self.provider_client(existing.provider).uses_mock(),
            };
            return Ok(StartFiatInvoicePayResponse {
                order: existing,
                quote,
                idempotent_replay: true,
            });
        }

        let invoice = self
            .payments
            .find_invoice_by_id(invoice_id)
            .await
            .map_err(map_payment_error)?
            .ok_or(ProviderError::Validation("invoice not found".into()))?;

        if invoice.status == InvoiceStatus::Paid {
            return Err(ProviderError::Conflict("invoice already paid".into()));
        }
        if invoice.status == InvoiceStatus::Expired || invoice.status == InvoiceStatus::Cancelled {
            return Err(ProviderError::Validation("invoice not payable".into()));
        }

        let merchant = self
            .payments
            .find_merchant_by_id(invoice.merchant_id)
            .await
            .map_err(map_payment_error)?
            .ok_or(ProviderError::Validation("merchant not found".into()))?;

        if merchant.owner_user_id == user_id {
            return Err(ProviderError::Validation(
                "cannot pay your own merchant invoice with fiat".into(),
            ));
        }

        let provider = FiatProvider::parse(&req.provider).ok_or_else(|| {
            ProviderError::Validation("provider must be transak".into())
        })?;

        let deposit = self
            .wallets
            .get_or_create_deposit_address(
                user_id,
                crate::internal::wallets::models::CreateAddressRequest {
                    asset: invoice.asset.clone(),
                    chain: invoice.chain.clone(),
                },
            )
            .await
            .map_err(map_wallet_error)?;

        let wallet_address = deposit.address.address.clone();
        let client = self.provider_client(provider);

        let quote = client
            .quote(
                &req.fiat_currency,
                None,
                &invoice.asset,
                &invoice.chain,
                Some(invoice.amount),
                Some(&wallet_address),
            )
            .await?;

        let order = self
            .repo
            .create_order(
                user_id,
                Some(invoice_id),
                provider,
                &quote,
                &wallet_address,
                &req.idempotency_key,
                quote.external_order_id.as_deref(),
            )
            .await?;

        let partner_ref = order.id.to_string();
        let checkout = order
            .checkout_url
            .as_deref()
            .map(|url| append_query_param(url, "partnerOrderId", &partner_ref));
        let order = self
            .repo
            .attach_partner_reference(order.id, &partner_ref, checkout.as_deref())
            .await?;

        Ok(StartFiatInvoicePayResponse {
            order,
            quote: to_quote_response(provider, quote),
            idempotent_replay: false,
        })
    }

    /// Dev/mock: mark order completed and settle invoice (deposit + wallet pay).
    pub async fn mock_complete_order(
        &self,
        user_id: Uuid,
        order_id: Uuid,
    ) -> ProviderResult<FiatConversionOrder> {
        if self.config.environment.is_production() {
            return Err(ProviderError::Forbidden);
        }
        if !self.config.providers.fiat_mock_mode && self.transak.is_configured() {
            return Err(ProviderError::Validation(
                "mock complete only available in fiat mock mode".into(),
            ));
        }

        let order = self.get_order(user_id, order_id).await?;
        self.complete_order_internal(order, "mock-purchase", "RELEASED")
            .await
    }

    pub async fn handle_transak_webhook(&self, body: serde_json::Value) -> ProviderResult<()> {
        transak::webhooks::verify_webhook_secret(
            self.transak.config(),
            self.config.environment,
            body.get("secret").and_then(|v| v.as_str()),
        )?;

        let notification = transak::webhooks::parse_webhook(&body)?;

        let order = self
            .resolve_order_for_webhook(
                FiatProvider::Transak,
                &notification.order_id,
                Some(&notification.raw),
            )
            .await?;

        self.complete_order_internal(
            order,
            &notification.order_id,
            notification.status.as_str(),
        )
        .await?;
        Ok(())
    }

    async fn complete_order_internal(
        &self,
        order: FiatConversionOrder,
        external_id: &str,
        status: &str,
    ) -> ProviderResult<FiatConversionOrder> {
        if order.status == FiatOrderStatus::Completed {
            return Ok(order);
        }

        if is_failure_status(status) {
            return self.repo.mark_failed(order.id).await;
        }

        if !is_success_status(status) {
            return self
                .repo
                .mark_processing(order.id, Some(external_id))
                .await;
        }

        let payer_wallet_id = self
            .wallets
            .get_default_wallet_id(order.user_id)
            .await
            .map_err(map_wallet_error)?;

        let deposit_key = format!(
            "fiat-{}:{}",
            order.provider.as_str(),
            external_id
        );
        let tx_hash = format!("fiat:{}:{}", order.provider.as_str(), external_id);

        self.wallets
            .record_confirmed_deposit(RecordDepositRequest {
                wallet_id: payer_wallet_id,
                asset: order.crypto_asset.clone(),
                chain: order.crypto_chain.clone(),
                tx_hash,
                confirmations: 1,
                idempotency_key: deposit_key,
                amount: order.crypto_amount,
                from_address: None,
                to_address: order.wallet_address.clone(),
                metadata: Some(serde_json::json!({
                    "fiat_order_id": order.id,
                    "provider": order.provider.as_str(),
                    "external_order_id": external_id,
                })),
            })
            .await
            .map_err(map_wallet_error)?;

        let mut payment_id = order.payment_id;
        if let Some(invoice_id) = order.invoice_id {
            if payment_id.is_none() {
                let method = payment_method_for_provider(order.provider);
                let idem = format!("fiat-payment:{}", order.idempotency_key);

                let (payment, _, _, _) = self
                    .payments
                    .complete_payment_with_wallet_transfer(
                        self.wallets.repository(),
                        invoice_id,
                        order.user_id,
                        payer_wallet_id,
                        self.clearing_wallet_id,
                        order.crypto_amount,
                        Decimal::ZERO,
                        &order.crypto_asset,
                        &order.crypto_chain,
                        method,
                        &idem,
                    )
                    .await
                    .map_err(map_payment_error)?;

                payment_id = Some(payment.id);
            }
        }

        self.repo.mark_completed(order.id, payment_id).await
    }

    async fn resolve_order_for_webhook(
        &self,
        provider: FiatProvider,
        external_id: &str,
        payload: Option<&serde_json::Value>,
    ) -> ProviderResult<FiatConversionOrder> {
        if let Some(order) = self.repo.find_by_external(provider, external_id).await? {
            return Ok(order);
        }

        if let Some(partner) = payload
            .and_then(|p| p.get("partnerOrderId"))
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
        {
            if let Some(order) = self.repo.find_by_id(partner).await? {
                return Ok(order);
            }
        }

        if let Ok(partner) = Uuid::parse_str(external_id) {
            if let Some(order) = self.repo.find_by_id(partner).await? {
                return Ok(order);
            }
        }

        Err(ProviderError::OrderNotFound)
    }

    fn provider_client(&self, _provider: FiatProvider) -> &dyn FiatOnRampProvider {
        &self.transak
    }
}

fn append_query_param(url: &str, key: &str, value: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{key}={value}")
}

#[derive(Clone)]
pub struct FiatConversionServiceHandle(pub Arc<FiatConversionService>);

impl FiatConversionServiceHandle {
    pub fn new(
        pool: PgPool,
        http: Client,
        config: Arc<Config>,
        wallets: WalletServiceHandle,
        clearing_wallet_id: Uuid,
    ) -> Self {
        Self(Arc::new(FiatConversionService::new(
            pool,
            http,
            config,
            wallets,
            clearing_wallet_id,
        )))
    }
}

impl std::ops::Deref for FiatConversionServiceHandle {
    type Target = FiatConversionService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn to_quote_response(provider: FiatProvider, quote: ProviderQuote) -> FiatQuoteResponse {
    FiatQuoteResponse {
        provider,
        fiat_currency: quote.fiat_currency,
        fiat_amount: quote.fiat_amount,
        crypto_asset: quote.crypto_asset,
        crypto_chain: quote.crypto_chain,
        crypto_amount: quote.crypto_amount,
        exchange_rate: quote.exchange_rate,
        fee_fiat: quote.fee_fiat,
        mock: quote.mock,
    }
}

fn payment_method_for_provider(_provider: FiatProvider) -> PaymentMethod {
    PaymentMethod::FiatTransak
}

fn is_success_status(status: &str) -> bool {
    matches!(
        status.to_uppercase().as_str(),
        "RELEASED"
            | "COMPLETED"
            | "COMPLETE"
            | "SUCCESS"
            | "PAYMENT_DONE"
            | "SETTLED"
    )
}

fn is_failure_status(status: &str) -> bool {
    matches!(
        status.to_uppercase().as_str(),
        "FAILED"
            | "CANCELLED"
            | "CANCELED"
            | "DECLINED"
            | "EXPIRED"
            | "REFUNDED"
    )
}

fn normalize_token(asset: &str) -> ProviderResult<String> {
    let v = asset.trim().to_uppercase();
    if v.len() < 2 || v.len() > 32 {
        return Err(ProviderError::Validation("invalid crypto asset".into()));
    }
    Ok(v)
}

fn normalize_chain(chain: &str) -> ProviderResult<String> {
    let v = chain.trim().to_lowercase();
    if v.len() < 3 || v.len() > 32 {
        return Err(ProviderError::Validation("invalid chain".into()));
    }
    Ok(v)
}

fn map_payment_error(err: PaymentError) -> ProviderError {
    match err {
        PaymentError::InsufficientBalance => {
            ProviderError::Validation("insufficient balance after fiat deposit".into())
        }
        PaymentError::InvoiceAlreadyPaid => ProviderError::Conflict("invoice already paid".into()),
        PaymentError::Validation(m) => ProviderError::Validation(m),
        PaymentError::Forbidden => ProviderError::Forbidden,
        PaymentError::Database(e) => ProviderError::Database(e),
        PaymentError::Internal(e) => ProviderError::Internal(e),
        other => ProviderError::Validation(other.to_string()),
    }
}

fn map_wallet_error(err: crate::internal::wallets::error::WalletError) -> ProviderError {
    match err {
        crate::internal::wallets::error::WalletError::InsufficientBalance => {
            ProviderError::Validation("insufficient wallet balance".into())
        }
        crate::internal::wallets::error::WalletError::Validation(m) => ProviderError::Validation(m),
        crate::internal::wallets::error::WalletError::Forbidden => ProviderError::Forbidden,
        crate::internal::wallets::error::WalletError::Database(e) => ProviderError::Database(e),
        crate::internal::wallets::error::WalletError::Internal(e) => ProviderError::Internal(e),
        other => ProviderError::Validation(other.to_string()),
    }
}
