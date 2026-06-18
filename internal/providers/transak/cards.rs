//! Transak payment methods (cards, bank transfer, etc.).

use reqwest::Client;

use crate::internal::providers::transak::config::TransakProviderConfig;
use crate::internal::providers::transak::error::{TransakError, TransakResult};
use crate::internal::providers::transak::models::PaymentMethodOption;

pub struct PaymentMethodsService {
    config: TransakProviderConfig,
    #[allow(dead_code)]
    http: Client,
}

impl PaymentMethodsService {
    pub fn new(http: Client, config: TransakProviderConfig) -> Self {
        Self { config, http }
    }

    pub async fn list_payment_methods(
        &self,
        fiat_currency: &str,
    ) -> TransakResult<Vec<PaymentMethodOption>> {
        if self.config.mock_mode || self.config.api_key.is_none() {
            return Ok(mock_methods(fiat_currency));
        }

        let _ = fiat_currency;
        Err(TransakError::Upstream(
            "Transak payment methods API not configured".into(),
        ))
    }
}

fn mock_methods(fiat_currency: &str) -> Vec<PaymentMethodOption> {
    vec![
        PaymentMethodOption {
            id: "credit_debit_card".into(),
            label: "Credit / Debit Card".into(),
            payment_method_type: "credit_debit_card".into(),
            enabled: true,
        },
        PaymentMethodOption {
            id: "sepa_bank_transfer".into(),
            label: "SEPA Bank Transfer".into(),
            payment_method_type: "sepa_bank_transfer".into(),
            enabled: matches!(fiat_currency, "EUR" | "GBP"),
        },
        PaymentMethodOption {
            id: "apple_pay".into(),
            label: "Apple Pay".into(),
            payment_method_type: "apple_pay".into(),
            enabled: fiat_currency == "USD" || fiat_currency == "EUR",
        },
    ]
}
