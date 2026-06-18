use rust_decimal::Decimal;

use crate::internal::swaps::config::SwapsConfig;
use crate::internal::swaps::models::FeeBreakdown;
use crate::internal::swaps::traits::LiquidityQuote;

/// Applies gateway platform fee on top of provider quote output.
pub fn apply_platform_fee(
    config: &SwapsConfig,
    quote: &LiquidityQuote,
) -> (Decimal, FeeBreakdown) {
    let bps = Decimal::from(config.platform_fee_bps);
    let platform_amount = (quote.to_amount * bps / Decimal::from(10_000)).round_dp(18);
    let net_output = quote.to_amount - platform_amount;

    let total = platform_amount + quote.fee_provider + quote.fee_network;

    (
        net_output,
        FeeBreakdown {
            platform_bps: config.platform_fee_bps,
            platform_amount,
            provider_amount: quote.fee_provider,
            network_amount: quote.fee_network,
            total_amount: total,
        },
    )
}
