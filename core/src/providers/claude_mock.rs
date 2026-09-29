//! Claude Code mock provider. Real source: none exists — Claude Code
//! quota is server-authoritative, only surfaced via the interactive
//! `/usage` slash command, no local or non-interactive query mode.
//! TUI scraping is forbidden. This mock never calls anything external;
//! it exists only to exercise the port end-to-end (docs/architecture.md).

use crate::provider::{ProviderError, ProviderStatus, UsageProvider, UsageSnapshot};

pub struct ClaudeMockProvider;

impl UsageProvider for ClaudeMockProvider {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn fetch_usage(&self) -> Result<UsageSnapshot, ProviderError> {
        Ok(UsageSnapshot {
            provider: self.id().to_string(),
            status: ProviderStatus::Pending,
            session_usage_percent: Some(12.0),
            session_reset_label: Some("resets in 5h (sample)".to_string()),
            weekly_usage_percent: Some(34.0),
            weekly_reset_label: Some("resets in 6d (sample)".to_string()),
            counts: Vec::new(),
            note: Some("mock data, no live source exists".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_is_deterministic_across_calls() {
        let provider = ClaudeMockProvider;
        let first = provider.fetch_usage().unwrap();
        let second = provider.fetch_usage().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn mock_reports_pending_not_available() {
        let provider = ClaudeMockProvider;
        let snapshot = provider.fetch_usage().unwrap();
        assert_eq!(snapshot.status, ProviderStatus::Pending);
        assert!(snapshot.note.is_some());
    }

    #[test]
    fn mock_identity_fields() {
        let provider = ClaudeMockProvider;
        assert_eq!(provider.id(), "claude-code");
        assert_eq!(provider.display_name(), "Claude Code");
    }
}
