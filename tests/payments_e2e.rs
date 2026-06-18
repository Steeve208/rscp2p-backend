//! E2E: pay invoice (wallet) + settlement liquidation to merchant.
//!
//! Run: `RSC_E2E=1 cargo test --test payments_e2e -- --nocapture`
//! Requires DATABASE_URL, REDIS_URL, JWT_SECRET (see `.env`).

mod e2e_common;

use rust_decimal::Decimal;

use e2e_common::{
    authed_json, available_balance, build_router, e2e_enabled, fund_wallet, register_and_token,
    setup_app, unique_email,
};

#[tokio::test]
#[ignore = "requires RSC_E2E=1 plus DATABASE_URL and REDIS_URL"]
async fn pay_invoice_then_settlement_moves_clearing_to_merchant() {
    assert!(e2e_enabled(), "set RSC_E2E=1 to run this test");

    let state = setup_app().await;
    let app = build_router(state.clone());

    let merchant_email = unique_email("merchant");
    let payer_email = unique_email("payer");
    let (merchant_user_id, merchant_token) = register_and_token(&app, &merchant_email).await;
    let (payer_user_id, payer_token) = register_and_token(&app, &payer_email).await;

    let _ = merchant_user_id;
    let _ = payer_user_id;

    fund_wallet(
        &state,
        payer_user_id,
        "RSC",
        "rsc-mainnet",
        Decimal::new(500, 0),
        "e2e-payer-fund",
    )
    .await;

    authed_json(
        &app,
        "POST",
        "/payments/merchants",
        &merchant_token,
        Some(serde_json::json!({
            "display_name": "E2E Shop",
            "settlement_asset": "RSC",
            "settlement_chain": "rsc-mainnet"
        })),
    )
    .await;

    let invoice = authed_json(
        &app,
        "POST",
        "/payments/invoices",
        &merchant_token,
        Some(serde_json::json!({
            "amount": "100",
            "asset": "RSC",
            "chain": "rsc-mainnet",
            "description": "e2e test invoice"
        })),
    )
    .await;

    let invoice_id = invoice["invoice"]["id"].as_str().expect("invoice id");

    authed_json(
        &app,
        "POST",
        &format!("/payments/invoices/{invoice_id}/pay"),
        &payer_token,
        Some(serde_json::json!({
            "idempotency_key": "e2e-pay-001",
            "method": "instant"
        })),
    )
    .await;

    let merchant_wallet_id = state
        .wallets
        .get_default_wallet_id(merchant_user_id)
        .await
        .expect("merchant wallet");

    let merchant_balance = available_balance(
        &state,
        merchant_wallet_id,
        "RSC",
        "rsc-mainnet",
    )
    .await;
    assert_eq!(
        merchant_balance,
        Decimal::ZERO,
        "merchant should not receive funds until settlement"
    );

    let clearing_balance = available_balance(
        &state,
        state.clearing_wallet_id,
        "RSC",
        "rsc-mainnet",
    )
    .await;
    assert_eq!(
        clearing_balance,
        Decimal::new(100, 0),
        "clearing wallet should hold payment"
    );

    let settlement = authed_json(
        &app,
        "POST",
        "/payments/merchants/me/settlements",
        &merchant_token,
        Some(serde_json::json!({
            "idempotency_key": "e2e-settlement-001"
        })),
    )
    .await;

    assert_eq!(
        settlement["settlement"]["status"].as_str().expect("status"),
        "completed"
    );
    assert!(settlement["settlement"]["wallet_journal_id"]
        .as_str()
        .is_some());

    let merchant_after = available_balance(
        &state,
        merchant_wallet_id,
        "RSC",
        "rsc-mainnet",
    )
    .await;
    assert_eq!(merchant_after, Decimal::new(100, 0));

    let clearing_after = available_balance(
        &state,
        state.clearing_wallet_id,
        "RSC",
        "rsc-mainnet",
    )
    .await;
    assert_eq!(clearing_after, Decimal::ZERO);
}
