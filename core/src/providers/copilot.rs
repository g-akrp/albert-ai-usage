//! Copilot adapter. Source: `gh api copilot_internal/user` (GitHub CLI,
//! internal/undocumented endpoint but no special scope needed — a
//! plain `gh auth login` token works). Field shapes confirmed by a
//! live call, not research alone (docs/data-source/copilot.md).
//! V0.1.0 default: reads a sanitized fixture, no live call. Live
//! capture is a separate, opt-in step (`ALBERT_LIVE_COPILOT=1`,
//! `make run-live-copilot`, see `crate::process`).
//!
//! Allowlist only: copilot_plan, and each quota_snapshots.*.
//! {entitlement,credits_used}. Everything else in the real response
//! (id, login, analytics_tracking_id, endpoints, enterprise/org lists,
//! and any unknown field) is dropped by construction — the deserialize
//! target below has no field for them.

use serde::Deserialize;
use std::collections::BTreeMap;

use crate::provider::{ProviderError, ProviderStatus, UsageCount, UsageProvider, UsageSnapshot};

const FIXTURE: &str = include_str!("../../tests/fixtures/copilot_user.json");

#[derive(Debug, Deserialize)]
struct RawResponse {
    #[serde(rename = "copilot_plan")]
    copilot_plan: Option<String>,
    #[serde(rename = "quota_snapshots")]
    quota_snapshots: BTreeMap<String, RawQuota>,
}

#[derive(Debug, Deserialize)]
struct RawQuota {
    entitlement: i64,
    #[serde(rename = "credits_used")]
    credits_used: i64,
}

/// Parses a raw `copilot_internal/user` response body into the
/// normalized `UsageSnapshot`. Pure function — where the JSON came
/// from (fixture today, a live `gh` call later) is the caller's
/// concern. Iterates `quota_snapshots` in a fixed (sorted) key order
/// so output is deterministic regardless of the map's wire order.
pub fn parse_copilot_user(raw_json: &str) -> Result<UsageSnapshot, ProviderError> {
    let parsed: RawResponse = serde_json::from_str(raw_json)
        .map_err(|_| ProviderError("could not parse copilot user response".to_string()))?;

    let counts = parsed
        .quota_snapshots
        .iter()
        .map(|(label, quota)| UsageCount {
            label: label.clone(),
            used: quota.credits_used.max(0) as u32,
            limit: quota.entitlement.max(0) as u32,
        })
        .collect();

    let note = match &parsed.copilot_plan {
        Some(plan) => format!("plan: {plan}"),
        None => "plan unknown".to_string(),
    };

    Ok(UsageSnapshot {
        provider: "copilot".to_string(),
        status: ProviderStatus::Pending,
        session_usage_percent: None,
        session_reset_label: None,
        weekly_usage_percent: None,
        weekly_reset_label: None,
        counts,
        note: Some(note),
    })
}

pub struct CopilotProvider;

impl UsageProvider for CopilotProvider {
    fn id(&self) -> &'static str {
        "copilot"
    }

    fn display_name(&self) -> &'static str {
        "GitHub Copilot"
    }

    fn fetch_usage(&self) -> Result<UsageSnapshot, ProviderError> {
        // Off by default. Set ALBERT_LIVE_COPILOT=1
        // (`make run-live-copilot`) to call the real `gh` CLI instead
        // of reading the fixture.
        if std::env::var("ALBERT_LIVE_COPILOT").as_deref() == Ok("1") {
            return Ok(match crate::process::call_copilot_user() {
                Ok(raw) => match parse_copilot_user(&raw) {
                    Ok(mut snapshot) => {
                        snapshot.status = ProviderStatus::Available;
                        snapshot.note = Some(format!(
                            "live gh response; {}",
                            snapshot.note.unwrap_or_default()
                        ));
                        snapshot
                    }
                    Err(_) => unsupported("live response did not match the expected shape"),
                },
                Err(reason) => unsupported(&reason),
            });
        }

        parse_copilot_user(FIXTURE)
    }
}

fn unsupported(reason: &str) -> UsageSnapshot {
    UsageSnapshot {
        provider: "copilot".to_string(),
        status: ProviderStatus::Unsupported,
        session_usage_percent: None,
        session_reset_label: None,
        weekly_usage_percent: None,
        weekly_reset_label: None,
        counts: Vec::new(),
        note: Some(reason.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_three_quota_categories() {
        let snapshot = parse_copilot_user(FIXTURE).unwrap();
        assert_eq!(snapshot.provider, "copilot");
        assert_eq!(snapshot.status, ProviderStatus::Pending);
        assert_eq!(snapshot.counts.len(), 3);
    }

    #[test]
    fn maps_premium_interactions_used_limit_correctly() {
        let snapshot = parse_copilot_user(FIXTURE).unwrap();
        let premium = snapshot
            .counts
            .iter()
            .find(|c| c.label == "premium_interactions")
            .unwrap();
        assert_eq!(premium.used, 42);
        assert_eq!(premium.limit, 300);
    }

    #[test]
    fn note_mentions_plan_never_raw_identity_fields() {
        let snapshot = parse_copilot_user(FIXTURE).unwrap();
        let note = snapshot.note.unwrap();
        assert!(note.contains("individual"));
        assert!(!note.contains("should_never_appear"));
    }

    #[test]
    fn malformed_json_returns_sanitized_error() {
        let err = parse_copilot_user("{not json").unwrap_err();
        assert_eq!(err.0, "could not parse copilot user response");
    }

    #[test]
    fn missing_quota_snapshots_returns_sanitized_error() {
        let err = parse_copilot_user(r#"{"copilot_plan":"individual"}"#).unwrap_err();
        assert_eq!(err.0, "could not parse copilot user response");
    }

    #[test]
    fn copilot_provider_identity() {
        let provider = CopilotProvider;
        assert_eq!(provider.id(), "copilot");
        assert_eq!(provider.display_name(), "GitHub Copilot");
    }
}
