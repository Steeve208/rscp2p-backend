use std::sync::Arc;

use crate::internal::swaps::models::SwapProviderInfo;
use crate::internal::swaps::traits::SwapLiquidityProvider;

pub struct SwapProviderRegistry {
    providers: Vec<Arc<dyn SwapLiquidityProvider>>,
}

impl SwapProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Box<dyn SwapLiquidityProvider>) {
        self.providers.push(Arc::from(provider));
    }

    pub fn list(&self) -> Vec<SwapProviderInfo> {
        self.providers
            .iter()
            .map(|p| SwapProviderInfo {
                id: p.id().to_string(),
                venue_kind: p.venue_kind(),
                configured: p.is_configured(),
                mock_mode: p.uses_mock(),
                supported_pairs: p.supported_pairs().to_vec(),
            })
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn SwapLiquidityProvider>> {
        self.providers
            .iter()
            .find(|p| p.id() == id)
            .cloned()
    }

    pub fn providers_for_pair<'a>(
        &'a self,
        from: &str,
        to: &str,
        filter_id: Option<&str>,
    ) -> Vec<Arc<dyn SwapLiquidityProvider>> {
        self.providers
            .iter()
            .filter(|p| {
                if let Some(id) = filter_id {
                    if p.id() != id {
                        return false;
                    }
                }
                p.supports_pair(from, to)
            })
            .cloned()
            .collect()
    }

    pub fn all(&self) -> &[Arc<dyn SwapLiquidityProvider>] {
        &self.providers
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl Default for SwapProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
