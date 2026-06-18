//! Wallet Service — Business logic for the financial core.
//!
//! Core invariants this service must protect:
//! - Never move money without a corresponding double-entry ledger record
//! - All external movements (deposit/withdrawal) must be idempotent
//! - Balances must be derivable from the ledger (or reconciled against it)

use std::sync::Arc;

use redis::aio::ConnectionManager;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::internal::wallets::error::{WalletError, WalletResult};
use crate::internal::wallets::models::{
    BroadcastWithdrawalRequest, BroadcastWithdrawalResponse, CreateAddressRequest,
    DepositAddressResponse, DepositTarget, EnsureDefaultWalletResponse, LedgerEntry,
    RecordDepositRequest, RecordDepositResponse, RequestWithdrawalRequest,
    RequestWithdrawalResponse, Wallet, WalletAddress, WalletBalance, WalletTransaction,
};
use crate::internal::wallets::repository::WalletRepository;

#[derive(Clone)]
pub struct WalletService {
    repo: WalletRepository,
    #[allow(dead_code)]
    redis: ConnectionManager,
}

impl WalletService {
    pub fn new(pool: PgPool, redis: ConnectionManager) -> Self {
        Self {
            repo: WalletRepository::new(pool),
            redis,
        }
    }

    pub async fn list_wallets(&self, user_id: Uuid) -> WalletResult<Vec<Wallet>> {
        self.repo.list_wallets_by_user(user_id).await
    }

    pub async fn ensure_default_wallet(
        &self,
        user_id: Uuid,
    ) -> WalletResult<EnsureDefaultWalletResponse> {
        let wallet = self.repo.ensure_default_wallet(user_id).await?;
        Ok(EnsureDefaultWalletResponse { wallet })
    }

    pub async fn get_default_wallet_id(&self, user_id: Uuid) -> WalletResult<Uuid> {
        self.repo.get_default_wallet_id(user_id).await
    }

    pub async fn wallet_belongs_to_user(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
    ) -> WalletResult<bool> {
        self.repo.wallet_belongs_to_user(wallet_id, user_id).await
    }

    pub(crate) fn repository(&self) -> &WalletRepository {
        &self.repo
    }

    pub async fn ensure_clearing_wallet(&self) -> WalletResult<Uuid> {
        self.repo.ensure_clearing_wallet().await
    }

    pub async fn get_wallet_balance(
        &self,
        wallet_id: Uuid,
        asset: &str,
        chain: &str,
    ) -> WalletResult<Option<WalletBalance>> {
        self.repo.get_balance_for_wallet(wallet_id, asset, chain).await
    }

    pub async fn list_balances(&self, user_id: Uuid) -> WalletResult<Vec<WalletBalance>> {
        self.repo.list_balances_by_user(user_id).await
    }

    pub async fn list_addresses(&self, user_id: Uuid) -> WalletResult<Vec<WalletAddress>> {
        self.repo.list_addresses_by_user(user_id).await
    }

    pub async fn get_or_create_deposit_address(
        &self,
        user_id: Uuid,
        req: CreateAddressRequest,
    ) -> WalletResult<DepositAddressResponse> {
        req.validate()
            .map_err(|e| WalletError::Validation(e.to_string()))?;

        let asset = normalize_asset(&req.asset)?;
        let chain = normalize_chain(&req.chain)?;
        let address = generate_controlled_deposit_address(&asset, &chain);
        let (wallet, address) = self
            .repo
            .get_or_create_deposit_address(user_id, &asset, &chain, &address)
            .await?;

        Ok(DepositAddressResponse { wallet, address })
    }

    pub async fn list_transactions(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> WalletResult<Vec<WalletTransaction>> {
        self.repo
            .list_transactions_by_user(user_id, normalize_limit(limit))
            .await
    }

    pub async fn list_ledger_entries(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> WalletResult<Vec<LedgerEntry>> {
        self.repo
            .list_ledger_entries_by_user(user_id, normalize_limit(limit))
            .await
    }

    /// Called by future blockchain listeners/workers after enough confirmations.
    /// This is intentionally not exposed as a public user action.
    pub async fn find_deposit_target_by_address(
        &self,
        address: &str,
    ) -> WalletResult<Option<DepositTarget>> {
        self.repo.find_deposit_target_by_address(address).await
    }

    pub async fn request_withdrawal(
        &self,
        user_id: Uuid,
        req: RequestWithdrawalRequest,
    ) -> WalletResult<RequestWithdrawalResponse> {
        req.validate()
            .map_err(|e| WalletError::Validation(e.to_string()))?;

        if req.amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        let fee = req.fee.unwrap_or(Decimal::ZERO);
        if fee < Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        let asset = normalize_asset(&req.asset)?;
        let chain = normalize_chain(&req.chain)?;

        let wallet_id = match req.wallet_id {
            Some(id) => {
                if !self.repo.wallet_belongs_to_user(id, user_id).await? {
                    return Err(WalletError::Forbidden);
                }
                id
            }
            None => self.repo.get_default_wallet_id(user_id).await?,
        };

        let (transaction, idempotent_replay) = self
            .repo
            .request_withdrawal(user_id, wallet_id, &req, &asset, &chain, fee)
            .await?;

        Ok(RequestWithdrawalResponse {
            transaction,
            idempotent_replay,
        })
    }

    pub async fn cancel_withdrawal(
        &self,
        user_id: Uuid,
        transaction_id: Uuid,
    ) -> WalletResult<WalletTransaction> {
        self.repo.cancel_withdrawal(user_id, transaction_id).await
    }

    pub async fn get_withdrawal(
        &self,
        user_id: Uuid,
        transaction_id: Uuid,
    ) -> WalletResult<WalletTransaction> {
        self.repo
            .get_transaction_for_user(user_id, transaction_id)
            .await?
            .ok_or(WalletError::WalletNotFound)
    }

    pub async fn broadcast_withdrawal(
        &self,
        user_id: Uuid,
        transaction_id: Uuid,
        req: BroadcastWithdrawalRequest,
        blockchain: &crate::internal::blockchain::BlockchainServiceHandle,
    ) -> WalletResult<BroadcastWithdrawalResponse> {
        req.validate()
            .map_err(|e| WalletError::Validation(e.to_string()))?;

        let tx_hash = blockchain
            .broadcast_transaction(crate::internal::blockchain::models::BroadcastTxRequest {
                raw_tx_hex: req.raw_tx_hex,
            })
            .await
            .map_err(|e| WalletError::Validation(format!("broadcast failed: {e}")))?;

        let transaction = self
            .repo
            .mark_withdrawal_broadcast(user_id, transaction_id, &tx_hash.tx_hash)
            .await?;

        Ok(BroadcastWithdrawalResponse {
            transaction,
            tx_hash: tx_hash.tx_hash,
        })
    }

    pub async fn list_confirming_withdrawals(
        &self,
        limit: i64,
    ) -> WalletResult<Vec<WalletTransaction>> {
        self.repo.list_confirming_withdrawals(limit).await
    }

    pub async fn finalize_confirmed_withdrawal(
        &self,
        transaction_id: Uuid,
        tx_hash: &str,
        confirmations: i32,
    ) -> WalletResult<Option<WalletTransaction>> {
        self.repo
            .finalize_confirmed_withdrawal(transaction_id, tx_hash, confirmations)
            .await
    }

    pub async fn fail_withdrawal(
        &self,
        transaction_id: Uuid,
        tx_hash: &str,
    ) -> WalletResult<Option<WalletTransaction>> {
        self.repo.fail_withdrawal(transaction_id, tx_hash).await
    }

    pub async fn record_confirmed_deposit(
        &self,
        mut req: RecordDepositRequest,
    ) -> WalletResult<RecordDepositResponse> {
        req.validate()
            .map_err(|e| WalletError::Validation(e.to_string()))?;

        if req.amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        req.asset = normalize_asset(&req.asset)?;
        req.chain = normalize_chain(&req.chain)?;
        self.repo.record_confirmed_deposit(&req).await
    }
}

#[derive(Clone)]
pub struct WalletServiceHandle(pub Arc<WalletService>);

impl WalletServiceHandle {
    pub fn new(pool: PgPool, redis: ConnectionManager) -> Self {
        Self(Arc::new(WalletService::new(pool, redis)))
    }
}

impl std::ops::Deref for WalletServiceHandle {
    type Target = WalletService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 200)
}

fn normalize_asset(asset: &str) -> WalletResult<String> {
    let normalized = asset.trim().to_uppercase();
    if normalized.len() < 2
        || normalized.len() > 32
        || !normalized
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err(WalletError::Validation(
            "asset must be 2-32 uppercase alphanumeric characters".into(),
        ));
    }
    Ok(normalized)
}

fn normalize_chain(chain: &str) -> WalletResult<String> {
    let normalized = chain.trim().to_lowercase();
    if normalized.len() < 3
        || normalized.len() > 32
        || !normalized
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(WalletError::Validation(
            "chain must be 3-32 lowercase alphanumeric or hyphen characters".into(),
        ));
    }
    Ok(normalized)
}

fn generate_controlled_deposit_address(asset: &str, chain: &str) -> String {
    format!(
        "rscdep_{}_{}_{}",
        chain.replace('-', ""),
        asset.to_lowercase(),
        Uuid::new_v4().simple()
    )
}
