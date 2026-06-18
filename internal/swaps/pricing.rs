use rust_decimal::Decimal;

use crate::internal::swaps::config::SwapsConfig;
use crate::internal::swaps::error::{SwapError, SwapResult};
use crate::internal::swaps::fees::apply_platform_fee;
use crate::internal::swaps::models::{
    FeeBreakdown, ProviderQuoteView, SwapPair, SwapQuoteRequest, SwapQuoteResponse,
};
use crate::internal::swaps::registry::SwapProviderRegistry;
use crate::internal::swaps::routing::{rank_quotes, RoutedQuote};
use crate::internal::swaps::traits::{LiquidityQuote, QuoteRequest};

pub struct PricingEngine;

impl PricingEngine {
    pub async fn quote(
        registry: &SwapProviderRegistry,
        config: &SwapsConfig,
        request: SwapQuoteRequest,
    ) -> SwapResult<SwapQuoteResponse> {
        let pair = normalize_pair(&request)?;
        validate_amounts(request.from_amount, request.to_amount)?;

        let quote_req = QuoteRequest {
            pair: pair.clone(),
            from_amount: request.from_amount,
            to_amount: request.to_amount,
            slippage_bps: request.slippage_bps,
        };

        let providers = registry.providers_for_pair(
            &pair.from_asset,
            &pair.to_asset,
            request.provider_id.as_deref(),
        );

        if providers.is_empty() {
            return Err(SwapError::NoRoute);
        }

        let mut routed = Vec::new();
        for provider in providers {
            match provider.quote(&quote_req).await {
                Ok(q) => {
                    let (net_to, fees) = apply_platform_fee(config, &q);
                    routed.push(RoutedQuote {
                        provider_id: provider.id().to_string(),
                        venue_kind: provider.venue_kind(),
                        quote: q,
                        net_to_amount: net_to,
                        fees,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        provider = provider.id(),
                        error = %e,
                        "swap provider quote failed"
                    );
                }
            }
        }

        if routed.is_empty() {
            return Err(SwapError::NoRoute);
        }

        let ranked = rank_quotes(routed);
        let best = to_view(&ranked[0]);
        let alternatives = ranked.iter().skip(1).map(to_view).collect();

        Ok(SwapQuoteResponse {
            pair,
            fees: ranked[0].fees.clone(),
            slippage_bps: request.slippage_bps,
            best,
            alternatives,
        })
    }
}

fn to_view(r: &RoutedQuote) -> ProviderQuoteView {
    ProviderQuoteView {
        provider_id: r.provider_id.clone(),
        venue_kind: r.venue_kind,
        from_asset: r.quote.from_asset.clone(),
        to_asset: r.quote.to_asset.clone(),
        from_amount: r.quote.from_amount,
        to_amount: r.net_to_amount,
        exchange_rate: r.quote.exchange_rate,
        fee_provider: r.fees.provider_amount,
        fee_network: r.fees.network_amount,
        mock: r.quote.mock,
    }
}

fn normalize_pair(req: &SwapQuoteRequest) -> SwapResult<SwapPair> {
    let from = req.from_asset.trim().to_uppercase();
    let to = req.to_asset.trim().to_uppercase();
    let pair = SwapPair {
        from_asset: from.clone(),
        to_asset: to.clone(),
        from_chain: req.from_chain.as_ref().map(|c| c.trim().to_lowercase()),
        to_chain: req.to_chain.as_ref().map(|c| c.trim().to_lowercase()),
    };

    let supported = crate::internal::swaps::models::supported_pairs()
        .into_iter()
        .any(|p| crate::internal::swaps::models::pairs_match(&p, &from, &to));

    if !supported {
        return Err(SwapError::UnsupportedPair);
    }

    Ok(pair)
}

fn validate_amounts(
    from_amount: Option<Decimal>,
    to_amount: Option<Decimal>,
) -> SwapResult<()> {
    match (from_amount, to_amount) {
        (Some(f), _) if f > Decimal::ZERO => Ok(()),
        (_, Some(t)) if t > Decimal::ZERO => Ok(()),
        _ => Err(SwapError::Validation(
            "from_amount or to_amount required".into(),
        )),
    }
}
