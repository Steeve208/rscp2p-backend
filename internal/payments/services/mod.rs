//! Payment services — merchant system, payment engine, settlement logic.
//!
//! # Separation
//! - **Merchant**: onboarding comercios, vinculación wallet futura
//! - **Engine**: crear facturas, QR, pagos instantáneos
//! - **Settlement**: agrupar pagos completados y liquidar al comercio
//!
//! # Wallet integration
//! - **Pay**: payer → settlement clearing wallet (fondos retenidos).
//! - **Settlement**: clearing → merchant wallet (liquidación explícita en ledger).

use std::sync::Arc;

use chrono::{Duration, Utc};
use redis::aio::ConnectionManager;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::internal::payments::error::{PaymentError, PaymentResult};
use crate::internal::payments::models::{
    CreateInvoiceRequest, CreateInvoiceResponse, InvoicePublicView, InvoiceStatus,
    MerchantResponse, PayInvoiceRequest, PayInvoiceResponse, PaymentMethod, QrPaymentPayload,
    RegisterMerchantRequest, RequestSettlementRequest, SettlementResponse,
};
use crate::internal::payments::repository::PaymentRepository;
use crate::internal::wallets::error::WalletError;
use crate::internal::wallets::services::WalletServiceHandle;

const QR_SCHEME: &str = "rscpay";

#[derive(Clone)]
pub struct PaymentService {
    repo: PaymentRepository,
    wallets: WalletServiceHandle,
    clearing_wallet_id: Uuid,
    #[allow(dead_code)]
    redis: ConnectionManager,
}

impl PaymentService {
    pub fn new(
        pool: PgPool,
        redis: ConnectionManager,
        wallets: WalletServiceHandle,
        clearing_wallet_id: Uuid,
    ) -> Self {
        Self {
            repo: PaymentRepository::new(pool),
            wallets,
            clearing_wallet_id,
            redis,
        }
    }

    // ==================== Merchant system ====================

    pub async fn register_merchant(
        &self,
        owner_user_id: Uuid,
        req: RegisterMerchantRequest,
    ) -> PaymentResult<MerchantResponse> {
        req.validate()
            .map_err(|e| PaymentError::Validation(e.to_string()))?;

        let settlement_asset = normalize_asset(req.settlement_asset.as_deref().unwrap_or("RSC"))?;
        let settlement_chain =
            normalize_chain(req.settlement_chain.as_deref().unwrap_or("rsc-mainnet"))?;

        let merchant = self
            .repo
            .create_merchant(owner_user_id, &req, &settlement_asset, &settlement_chain)
            .await?;

        Ok(MerchantResponse { merchant })
    }

    pub async fn get_my_merchant(&self, owner_user_id: Uuid) -> PaymentResult<MerchantResponse> {
        let merchant = self
            .repo
            .find_merchant_by_owner(owner_user_id)
            .await?
            .ok_or(PaymentError::MerchantNotFound)?;

        Ok(MerchantResponse { merchant })
    }

    // ==================== Payment engine ====================

    pub async fn create_invoice(
        &self,
        owner_user_id: Uuid,
        req: CreateInvoiceRequest,
    ) -> PaymentResult<CreateInvoiceResponse> {
        req.validate()
            .map_err(|e| PaymentError::Validation(e.to_string()))?;

        if req.amount <= Decimal::ZERO {
            return Err(PaymentError::InvalidAmount);
        }

        let merchant = self
            .repo
            .find_merchant_by_owner(owner_user_id)
            .await?
            .ok_or(PaymentError::MerchantNotFound)?;

        let asset = normalize_asset(&req.asset)?;
        let chain = normalize_chain(&req.chain)?;

        let expires_at = req
            .expires_in_minutes
            .map(|m| Utc::now() + Duration::minutes(m as i64));

        let reference_code = generate_reference_code();

        let invoice = self
            .repo
            .create_invoice(
                merchant.id,
                &reference_code,
                &req,
                &asset,
                &chain,
                expires_at,
            )
            .await?;

        let qr = build_qr_payload(&invoice, &merchant.id);

        Ok(CreateInvoiceResponse { invoice, qr })
    }

    pub async fn get_invoice_public(
        &self,
        reference_code: &str,
    ) -> PaymentResult<InvoicePublicView> {
        let (invoice, merchant_name) = self
            .repo
            .get_invoice_public_view(reference_code)
            .await?
            .ok_or(PaymentError::InvoiceNotFound)?;

        if invoice.status == InvoiceStatus::Expired {
            return Err(PaymentError::InvoiceExpired);
        }

        Ok(InvoicePublicView {
            reference_code: invoice.reference_code,
            merchant_display_name: merchant_name,
            amount: invoice.amount,
            asset: invoice.asset,
            chain: invoice.chain,
            description: invoice.description,
            status: invoice.status,
            expires_at: invoice.expires_at,
        })
    }

    pub async fn pay_invoice(
        &self,
        payer_user_id: Uuid,
        invoice_id: Uuid,
        req: PayInvoiceRequest,
    ) -> PaymentResult<PayInvoiceResponse> {
        req.validate()
            .map_err(|e| PaymentError::Validation(e.to_string()))?;

        let invoice = self
            .repo
            .find_invoice_by_id(invoice_id)
            .await?
            .ok_or(PaymentError::InvoiceNotFound)?;

        if invoice.status == InvoiceStatus::Paid {
            return Err(PaymentError::InvoiceAlreadyPaid);
        }

        let merchant = self
            .repo
            .find_merchant_by_id(invoice.merchant_id)
            .await?
            .ok_or(PaymentError::MerchantNotFound)?;

        if merchant.owner_user_id == payer_user_id {
            return Err(PaymentError::Validation(
                "cannot pay your own merchant invoice".into(),
            ));
        }

        let payer_wallet_id = self
            .wallets
            .get_default_wallet_id(payer_user_id)
            .await
            .map_err(map_wallet_error)?;

        let method = parse_method(req.method.as_deref())?;

        let (payment, invoice, idempotent_replay, wallet_transfer_idempotent_replay) = self
            .repo
            .complete_payment_with_wallet_transfer(
                self.wallets.repository(),
                invoice_id,
                payer_user_id,
                payer_wallet_id,
                self.clearing_wallet_id,
                invoice.amount,
                Decimal::ZERO,
                &invoice.asset,
                &invoice.chain,
                method,
                &req.idempotency_key,
            )
            .await?;

        Ok(PayInvoiceResponse {
            payment,
            invoice,
            idempotent_replay,
            wallet_transfer_idempotent_replay,
        })
    }

    pub async fn pay_invoice_by_reference(
        &self,
        payer_user_id: Uuid,
        reference_code: &str,
        req: PayInvoiceRequest,
    ) -> PaymentResult<PayInvoiceResponse> {
        let invoice = self
            .repo
            .find_invoice_by_reference(reference_code)
            .await?
            .ok_or(PaymentError::InvoiceNotFound)?;

        self.pay_invoice(payer_user_id, invoice.id, req).await
    }

    pub async fn get_qr_payload(
        &self,
        owner_user_id: Uuid,
        invoice_id: Uuid,
    ) -> PaymentResult<QrPaymentPayload> {
        let invoice = self
            .repo
            .find_invoice_by_id(invoice_id)
            .await?
            .ok_or(PaymentError::InvoiceNotFound)?;

        if !self
            .repo
            .merchant_owned_by(invoice.merchant_id, owner_user_id)
            .await?
        {
            return Err(PaymentError::Forbidden);
        }

        Ok(build_qr_payload(&invoice, &invoice.merchant_id))
    }

    // ==================== Settlement logic ====================

    pub async fn request_settlement(
        &self,
        owner_user_id: Uuid,
        req: RequestSettlementRequest,
    ) -> PaymentResult<SettlementResponse> {
        req.validate()
            .map_err(|e| PaymentError::Validation(e.to_string()))?;

        let merchant = self
            .repo
            .find_merchant_by_owner(owner_user_id)
            .await?
            .ok_or(PaymentError::MerchantNotFound)?;

        let merchant_wallet_id = resolve_merchant_wallet_id(&self.wallets, &merchant).await?;

        let (settlement, payment_count, wallet_transfer_idempotent_replay) = self
            .repo
            .create_settlement_with_wallet_transfer(
                self.wallets.repository(),
                merchant.id,
                merchant_wallet_id,
                self.clearing_wallet_id,
                &merchant.settlement_asset,
                &merchant.settlement_chain,
                req.period_start,
                req.period_end,
                &req.idempotency_key,
            )
            .await?;

        Ok(SettlementResponse {
            settlement,
            payment_count,
            wallet_transfer_idempotent_replay,
        })
    }

    pub async fn list_settlements(
        &self,
        owner_user_id: Uuid,
        limit: Option<i64>,
    ) -> PaymentResult<Vec<SettlementResponse>> {
        let merchant = self
            .repo
            .find_merchant_by_owner(owner_user_id)
            .await?
            .ok_or(PaymentError::MerchantNotFound)?;

        let settlements = self
            .repo
            .list_settlements(merchant.id, normalize_limit(limit))
            .await?;

        Ok(settlements
            .into_iter()
            .map(|settlement| SettlementResponse {
                settlement,
                payment_count: 0,
                wallet_transfer_idempotent_replay: false,
            })
            .collect())
    }
}

#[derive(Clone)]
pub struct PaymentServiceHandle(pub Arc<PaymentService>);

impl PaymentServiceHandle {
    pub fn new(
        pool: PgPool,
        redis: ConnectionManager,
        wallets: WalletServiceHandle,
        clearing_wallet_id: Uuid,
    ) -> Self {
        Self(Arc::new(PaymentService::new(
            pool,
            redis,
            wallets,
            clearing_wallet_id,
        )))
    }
}

impl std::ops::Deref for PaymentServiceHandle {
    type Target = PaymentService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn build_qr_payload(
    invoice: &crate::internal::payments::models::PaymentInvoice,
    merchant_id: &Uuid,
) -> QrPaymentPayload {
    QrPaymentPayload {
        scheme: QR_SCHEME.into(),
        reference_code: invoice.reference_code.clone(),
        merchant_id: *merchant_id,
        amount: invoice.amount,
        asset: invoice.asset.clone(),
        chain: invoice.chain.clone(),
        expires_at: invoice.expires_at,
    }
}

fn generate_reference_code() -> String {
    format!("RSC{}", Uuid::new_v4().simple())
}

fn normalize_asset(asset: &str) -> PaymentResult<String> {
    let normalized = asset.trim().to_uppercase();
    if normalized.len() < 2 || normalized.len() > 32 {
        return Err(PaymentError::Validation("invalid asset".into()));
    }
    Ok(normalized)
}

fn normalize_chain(chain: &str) -> PaymentResult<String> {
    let normalized = chain.trim().to_lowercase();
    if normalized.len() < 3 || normalized.len() > 32 {
        return Err(PaymentError::Validation("invalid chain".into()));
    }
    Ok(normalized)
}

fn parse_method(value: Option<&str>) -> PaymentResult<PaymentMethod> {
    match value.unwrap_or("instant") {
        "qr" => Ok(PaymentMethod::Qr),
        "instant" => Ok(PaymentMethod::Instant),
        "invoice" => Ok(PaymentMethod::Invoice),
        other => Err(PaymentError::Validation(format!("unknown method: {other}"))),
    }
}

fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 200)
}

async fn resolve_merchant_wallet_id(
    wallets: &WalletServiceHandle,
    merchant: &crate::internal::payments::models::Merchant,
) -> PaymentResult<Uuid> {
    match merchant.wallet_id {
        Some(wallet_id) => {
            if !wallets
                .wallet_belongs_to_user(wallet_id, merchant.owner_user_id)
                .await
                .map_err(map_wallet_error)?
            {
                return Err(PaymentError::Validation(
                    "merchant wallet does not belong to owner".into(),
                ));
            }
            Ok(wallet_id)
        }
        None => wallets
            .get_default_wallet_id(merchant.owner_user_id)
            .await
            .map_err(map_wallet_error),
    }
}

fn map_wallet_error(err: WalletError) -> PaymentError {
    match err {
        WalletError::InsufficientBalance => PaymentError::InsufficientBalance,
        WalletError::InvalidAmount => PaymentError::InvalidAmount,
        WalletError::Validation(msg) => PaymentError::Validation(msg),
        WalletError::Forbidden => PaymentError::Forbidden,
        WalletError::Database(e) => PaymentError::Database(e),
        WalletError::Internal(e) => PaymentError::Internal(e),
        other => PaymentError::Validation(other.to_string()),
    }
}
