#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use crate::internal::config::ProvidersConfig;
    use crate::internal::providers::models::FiatProvider;
    use crate::internal::providers::striga::config::{StrigaProviderConfig, DEFAULT_SANDBOX_BASE_URL};
    use crate::internal::providers::traits::FiatOnRampProvider;
    use crate::internal::providers::transak::{TransakEnvironment, TransakProvider, TransakProviderConfig};

    fn mock_providers_config() -> ProvidersConfig {
        ProvidersConfig {
            transak: TransakProviderConfig {
                api_key: None,
                secret: None,
                environment: TransakEnvironment::Staging,
                api_base_url: None,
                mock_mode: true,
                webhook_secret: None,
            },
            striga: StrigaProviderConfig {
                app_id: None,
                api_key: None,
                api_secret: None,
                ui_secret: None,
                base_url: DEFAULT_SANDBOX_BASE_URL.to_string(),
                mock_mode: true,
                webhook_secret: None,
            },
            fiat_mock_mode: true,
        }
    }

    #[test]
    fn parses_provider_names() {
        assert_eq!(FiatProvider::parse("transak"), Some(FiatProvider::Transak));
        assert_eq!(FiatProvider::parse("TRANSAK"), Some(FiatProvider::Transak));
        assert_eq!(FiatProvider::parse("ramp"), Some(FiatProvider::Transak));
        assert!(FiatProvider::parse("stripe").is_none());
    }

    #[tokio::test]
    async fn transak_mock_quote_returns_checkout_url() {
        let http = reqwest::Client::new();
        let client = TransakProvider::new(http, mock_providers_config().transak);
        assert!(client.uses_mock());

        let quote = client
            .quote(
                "EUR",
                Some(Decimal::new(100, 0)),
                "BTC",
                "ethereum",
                None,
                None,
            )
            .await
            .expect("mock quote");

        assert!(quote.checkout_url.as_ref().unwrap().contains("transak"));
    }
}
