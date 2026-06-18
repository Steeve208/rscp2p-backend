use crate::internal::swaps::models::{FeeBreakdown, LiquidityVenueKind};
use crate::internal::swaps::traits::LiquidityQuote;

#[derive(Debug, Clone)]
pub struct RoutedQuote {
    pub provider_id: String,
    pub venue_kind: LiquidityVenueKind,
    pub quote: LiquidityQuote,
    pub net_to_amount: rust_decimal::Decimal,
    pub fees: FeeBreakdown,
}

/// Best route = highest net output to the user after platform fees.
pub fn rank_quotes(mut quotes: Vec<RoutedQuote>) -> Vec<RoutedQuote> {
    quotes.sort_by(|a, b| b.net_to_amount.cmp(&a.net_to_amount));
    quotes
}
