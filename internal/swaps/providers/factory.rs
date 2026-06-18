use std::collections::HashMap;

use reqwest::Client;

use crate::internal::config::{SwapProviderSettings, SwapVenueKind};
use crate::internal::swaps::config::SwapsConfig;
use crate::internal::swaps::models::LiquidityVenueKind;
use crate::internal::swaps::providers::adapter::ConfiguredLiquidityAdapter;
use crate::internal::swaps::providers::settings::ProviderRuntimeConfig;
use crate::internal::swaps::registry::SwapProviderRegistry;
use crate::internal::swaps::traits::SwapLiquidityProvider;

/// Builds the provider registry from configuration only (orchestration stays agnostic).
pub fn build_registry(config: &SwapsConfig, http: Client) -> SwapProviderRegistry {
    let mut registry = SwapProviderRegistry::new();

    for settings in &config.providers {
        if let Some(provider) = instantiate_provider(settings, http.clone()) {
            registry.register(provider);
        }
    }

    registry
}

fn instantiate_provider(
    settings: &SwapProviderSettings,
    http: Client,
) -> Option<Box<dyn SwapLiquidityProvider>> {
    let runtime = ProviderRuntimeConfig {
        settings: settings.clone(),
    };

    if !runtime.is_configured() {
        tracing::warn!(
            provider = %settings.id,
            "swap provider skipped: not configured"
        );
        return None;
    }

    Some(Box::new(ConfiguredLiquidityAdapter::new(runtime, http)))
}

/// Parse `SWAP_PROVIDERS` entries: `id@DEX,id@CEX` or `id` (venue from env suffix).
pub fn parse_provider_specs(raw: &str) -> Vec<(String, SwapVenueKind)> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            let (id, kind) = if let Some((id, kind)) = entry.split_once('@') {
                (
                    id.trim().to_string(),
                    liquidity_venue_to_config(LiquidityVenueKind::parse(kind)?),
                )
            } else {
                let kind = infer_venue_from_id(entry).unwrap_or(SwapVenueKind::Dex);
                (entry.to_string(), kind)
            };
            if id.is_empty() {
                return None;
            }
            Some((id, kind))
        })
        .collect()
}

fn infer_venue_from_id(id: &str) -> Option<SwapVenueKind> {
    let lower = id.to_lowercase();
    if lower.contains("cex") || lower.contains("exchange") {
        Some(SwapVenueKind::Cex)
    } else if lower.contains("dex") || lower.contains("pool") {
        Some(SwapVenueKind::Dex)
    } else {
        None
    }
}

fn liquidity_venue_to_config(kind: LiquidityVenueKind) -> SwapVenueKind {
    match kind {
        LiquidityVenueKind::Dex => SwapVenueKind::Dex,
        LiquidityVenueKind::Cex => SwapVenueKind::Cex,
    }
}

pub fn load_provider_settings(
    specs: &[(String, SwapVenueKind)],
    mock_mode: bool,
) -> Vec<SwapProviderSettings> {
    specs
        .iter()
        .map(|(id, venue)| {
            let prefix = env_prefix(id);
            let api_key = crate::internal::config::env::optional(&format!("{prefix}_API_KEY"));
            let api_base_url =
                crate::internal::config::env::optional(&format!("{prefix}_API_BASE_URL"));
            let provider_mock = crate::internal::config::env::optional(&format!("{prefix}_MOCK_MODE"))
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or(mock_mode);

            SwapProviderSettings {
                id: id.clone(),
                venue_kind: *venue,
                api_key,
                api_base_url,
                mock_mode: provider_mock,
            }
        })
        .collect()
}

fn env_prefix(id: &str) -> String {
    format!(
        "SWAP_PROVIDER_{}",
        id.to_uppercase()
            .replace('-', "_")
            .replace('.', "_")
    )
}

pub fn merge_env_overrides(
    mut providers: Vec<SwapProviderSettings>,
) -> Vec<SwapProviderSettings> {
    let mut by_id: HashMap<String, SwapProviderSettings> = HashMap::new();
    for p in providers.drain(..) {
        by_id.insert(p.id.clone(), p);
    }
    by_id.into_values().collect()
}
