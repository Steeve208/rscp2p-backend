#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use crate::internal::swaps::models::supported_pairs;
    use crate::internal::swaps::providers::factory::parse_provider_specs;
    use crate::internal::swaps::routing::{rank_quotes, RoutedQuote};
    use crate::internal::swaps::models::{FeeBreakdown, LiquidityVenueKind};
    use crate::internal::swaps::traits::LiquidityQuote;

    #[test]
    fn catalog_includes_required_pairs() {
        let pairs = supported_pairs();
        assert!(pairs.iter().any(|p| p.from_asset == "BTC" && p.to_asset == "USDT"));
        assert!(pairs.iter().any(|p| p.from_asset == "RSC" && p.to_asset == "BTC"));
        assert!(pairs.iter().any(|p| p.from_asset == "ETH" && p.to_asset == "BRL"));
    }

    #[test]
    fn parses_provider_specs() {
        let specs = parse_provider_specs("pool-a@DEX,pool-b@CEX");
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].0, "pool-a");
        assert_eq!(specs[1].0, "pool-b");
    }

    #[test]
    fn routing_picks_highest_net_output() {
        let low = routed("a", Decimal::new(100, 0));
        let high = routed("b", Decimal::new(105, 0));
        let ranked = rank_quotes(vec![low, high]);
        assert_eq!(ranked[0].provider_id, "b");
    }

    fn routed(id: &str, net: Decimal) -> RoutedQuote {
        RoutedQuote {
            provider_id: id.into(),
            venue_kind: LiquidityVenueKind::Dex,
            quote: LiquidityQuote {
                from_asset: "BTC".into(),
                to_asset: "USDT".into(),
                from_amount: Decimal::ONE,
                to_amount: net,
                exchange_rate: net,
                fee_provider: Decimal::ZERO,
                fee_network: Decimal::ZERO,
                mock: true,
                raw: serde_json::json!({}),
            },
            net_to_amount: net,
            fees: FeeBreakdown {
                platform_bps: 25,
                platform_amount: Decimal::ZERO,
                provider_amount: Decimal::ZERO,
                network_amount: Decimal::ZERO,
                total_amount: Decimal::ZERO,
            },
        }
    }
}
