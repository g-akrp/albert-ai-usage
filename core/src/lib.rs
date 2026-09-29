pub mod provider;
pub mod providers;
pub mod registry;

use provider::{ProviderError, UsageSnapshot};

/// Formats one provider's result as a simple human-readable line block.
/// Pure function, no I/O — unit-testable independent of the CLI.
pub fn format_result(id: &str, result: &Result<UsageSnapshot, ProviderError>) -> String {
    match result {
        Ok(snapshot) => {
            let mut lines = vec![format!("{} — {:?}", id, snapshot.status)];
            if let Some(p) = snapshot.session_usage_percent {
                lines.push(format!(
                    "  session: {:.0}%{}",
                    p,
                    snapshot
                        .session_reset_label
                        .as_deref()
                        .map(|l| format!(" ({l})"))
                        .unwrap_or_default()
                ));
            }
            if let Some(p) = snapshot.weekly_usage_percent {
                lines.push(format!(
                    "  weekly: {:.0}%{}",
                    p,
                    snapshot
                        .weekly_reset_label
                        .as_deref()
                        .map(|l| format!(" ({l})"))
                        .unwrap_or_default()
                ));
            }
            for count in &snapshot.counts {
                lines.push(format!("  {}: {}/{}", count.label, count.used, count.limit));
            }
            if let Some(note) = &snapshot.note {
                lines.push(format!("  note: {note}"));
            }
            lines.join("\n")
        }
        Err(ProviderError(reason)) => format!("{id} — Error: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider::{ProviderStatus, UsageSnapshot};

    #[test]
    fn format_result_includes_provider_id_and_status() {
        let snapshot = UsageSnapshot {
            provider: "claude-code".into(),
            status: ProviderStatus::Pending,
            session_usage_percent: Some(12.0),
            session_reset_label: None,
            weekly_usage_percent: None,
            weekly_reset_label: None,
            counts: Vec::new(),
            note: Some("mock data, no live source exists".into()),
        };
        let out = format_result("claude-code", &Ok(snapshot));
        assert!(out.contains("claude-code"));
        assert!(out.contains("Pending"));
        assert!(out.contains("12%"));
        assert!(out.contains("mock data"));
    }

    #[test]
    fn format_result_shows_sanitized_error() {
        let out = format_result("x", &Err(ProviderError("sanitized failure".into())));
        assert!(out.contains("Error: sanitized failure"));
    }

    #[test]
    fn format_result_shows_used_limit_counts() {
        let snapshot = UsageSnapshot {
            provider: "copilot".into(),
            status: ProviderStatus::Available,
            session_usage_percent: None,
            session_reset_label: None,
            weekly_usage_percent: None,
            weekly_reset_label: None,
            counts: vec![provider::UsageCount {
                label: "premium_interactions".into(),
                used: 42,
                limit: 300,
            }],
            note: None,
        };
        let out = format_result("copilot", &Ok(snapshot));
        assert!(out.contains("premium_interactions: 42/300"));
    }
}
