//! Provider registry: holds registered providers, refreshes each in
//! isolation so one provider's failure never blocks another's result.

use crate::provider::{ProviderError, UsageProvider, UsageSnapshot};

pub struct ProviderRegistry {
    providers: Vec<Box<dyn UsageProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Box<dyn UsageProvider>) {
        self.providers.push(provider);
    }

    /// Calls `fetch_usage` on every registered provider, in registration
    /// order. One provider's `Err` never stops or skips another's call.
    pub fn refresh_all(&self) -> Vec<(String, Result<UsageSnapshot, ProviderError>)> {
        self.providers
            .iter()
            .map(|p| (p.id(), p.fetch_usage()))
            .collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OkProvider;
    impl UsageProvider for OkProvider {
        fn id(&self) -> String {
            "ok".to_string()
        }
        fn display_name(&self) -> String {
            "Ok Provider".to_string()
        }
        fn fetch_usage(&self) -> Result<UsageSnapshot, ProviderError> {
            Ok(UsageSnapshot {
                provider: "ok".into(),
                status: crate::provider::ProviderStatus::Available,
                session_usage_percent: Some(1.0),
                session_reset_label: None,
                weekly_usage_percent: None,
                weekly_reset_label: None,
                counts: Vec::new(),
                note: None,
            })
        }
    }

    struct FailProvider;
    impl UsageProvider for FailProvider {
        fn id(&self) -> String {
            "fail".to_string()
        }
        fn display_name(&self) -> String {
            "Fail Provider".to_string()
        }
        fn fetch_usage(&self) -> Result<UsageSnapshot, ProviderError> {
            Err(ProviderError("sanitized failure".into()))
        }
    }

    #[test]
    fn refresh_all_isolates_provider_failures_and_keeps_order() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(OkProvider));
        registry.register(Box::new(FailProvider));

        let results = registry.refresh_all();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "ok");
        assert!(results[0].1.is_ok());
        assert_eq!(results[1].0, "fail");
        assert!(results[1].1.is_err());
    }

    #[test]
    fn refresh_all_on_empty_registry_returns_empty() {
        let registry = ProviderRegistry::new();
        assert!(registry.refresh_all().is_empty());
    }
}
