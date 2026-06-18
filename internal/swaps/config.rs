use crate::internal::config::{env, ConfigError, SwapProviderSettings};
use crate::internal::swaps::providers::factory::{
    load_provider_settings, merge_env_overrides, parse_provider_specs,
};

#[derive(Debug, Clone)]
pub struct SwapsConfig {
    pub providers: Vec<SwapProviderSettings>,
    pub mock_mode: bool,
    pub platform_fee_bps: u32,
    pub default_slippage_bps: u32,
}

impl SwapsConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mock_mode = env::bool("SWAP_MOCK_MODE", true)?;
        let swap_providers_raw =
            env::with_default("SWAP_PROVIDERS", "liquidity-dex@DEX,liquidity-cex@CEX");
        let specs = parse_provider_specs(&swap_providers_raw);
        let providers = merge_env_overrides(load_provider_settings(&specs, mock_mode));
        let platform_fee_bps = env::u32("SWAP_PLATFORM_FEE_BPS", "25")?;
        let default_slippage_bps = env::u32("SWAP_DEFAULT_SLIPPAGE_BPS", "50")?;

        Ok(Self {
            providers,
            mock_mode,
            platform_fee_bps,
            default_slippage_bps,
        })
    }
}
