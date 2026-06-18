//! Transak widget URL builder — embedded checkout inside RSC Bank (white-label).

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::internal::providers::transak::api::TransakApiClient;
use crate::internal::providers::transak::config::TransakProviderConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetFlow {
    Buy,
    Sell,
}

impl WidgetFlow {
    fn products_availed(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
}

/// Builds a widget URL for iframe / WebView embedding in RSC Bank.
pub fn build_widget_url(
    config: &TransakProviderConfig,
    flow: WidgetFlow,
    fiat_currency: &str,
    crypto_asset: &str,
    crypto_chain: &str,
    fiat_amount: Option<Decimal>,
    crypto_amount: Option<Decimal>,
    wallet_address: Option<&str>,
    partner_order_id: Uuid,
    user_id: Uuid,
) -> String {
    let base = config.widget_base_url();
    let api_key = config
        .api_key
        .as_deref()
        .unwrap_or("mock");

    let network = TransakApiClient::map_chain(crypto_chain);
    let mut params: Vec<(String, String)> = vec![
        ("apiKey".to_string(), api_key.to_string()),
        ("productsAvailed".to_string(), flow.products_availed().to_string()),
        ("fiatCurrency".to_string(), fiat_currency.to_uppercase()),
        ("cryptoCurrencyCode".to_string(), crypto_asset.to_uppercase()),
        ("network".to_string(), network),
        ("partnerOrderId".to_string(), partner_order_id.to_string()),
        ("partnerCustomerId".to_string(), user_id.to_string()),
        ("hideMenu".to_string(), "true".to_string()),
        ("isAutoFillUserData".to_string(), "true".to_string()),
    ];

    if let Some(f) = fiat_amount.filter(|a| *a > Decimal::ZERO) {
        params.push(("fiatAmount".to_string(), f.to_string()));
    }
    if let Some(c) = crypto_amount.filter(|a| *a > Decimal::ZERO) {
        params.push(("cryptoAmount".to_string(), c.to_string()));
    }
    if let Some(addr) = wallet_address {
        if flow == WidgetFlow::Buy {
            params.push(("walletAddress".to_string(), addr.to_string()));
        }
    }

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoding_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{base}?{query}")
}

fn urlencoding_encode(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::providers::transak::TransakEnvironment;

    #[test]
    fn buy_widget_contains_partner_order() {
        let config = TransakProviderConfig {
            api_key: Some("test-key".into()),
            api_base_url: None,
            mock_mode: true,
            webhook_secret: None,
            secret: None,
            environment: TransakEnvironment::Staging,
        };
        let url = build_widget_url(
            &config,
            WidgetFlow::Buy,
            "EUR",
            "BTC",
            "ethereum",
            Some(Decimal::new(100, 0)),
            None,
            Some("0xabc"),
            Uuid::nil(),
            Uuid::nil(),
        );
        assert!(url.contains("productsAvailed=BUY"));
        assert!(url.contains("partnerOrderId="));
    }
}
