//! Codex adapter. Source: local `codex app-server`, stdio JSON-RPC v2,
//! method `account/rateLimits/read` after an initialize handshake.
//! Research-grade (docs/provider-source-proposals.md); this parser is
//! TDD'd against a sanitized fixture. V0.1.0 does not spawn the real
//! process — no live call is made, no auth-state file is touched. Live
//! capture is a separate, human-authorized step (see docs).
//!
//! Allowlist only: primary/secondary usedPercent, windowDurationMins,
//! resetsAt, planType, ordinaryUsageAllowed, rateLimitResetCredit.
//! Everything else in the raw response (accountId, credentials,
//! balances, upsell, unknown fields) is dropped by construction — the
//! deserialize target below has no field for them, so serde ignores
//! them; they never reach `UsageSnapshot`.

use serde::Deserialize;

use crate::provider::{ProviderError, ProviderStatus, UsageCount, UsageProvider, UsageSnapshot};

const FIXTURE: &str = include_str!("../../tests/fixtures/codex_rate_limits.json");

#[derive(Debug, Deserialize)]
struct RawResponse {
    #[serde(rename = "rateLimits")]
    rate_limits: RawRateLimits,
}

#[derive(Debug, Deserialize)]
struct RawRateLimits {
    primary: RawWindow,
    secondary: RawWindow,
    #[serde(rename = "planType")]
    plan_type: Option<String>,
    #[serde(rename = "ordinaryUsageAllowed")]
    ordinary_usage_allowed: Option<bool>,
    #[serde(rename = "rateLimitResetCredit")]
    rate_limit_reset_credit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawWindow {
    #[serde(rename = "usedPercent")]
    used_percent: f32,
    #[serde(rename = "windowDurationMins")]
    window_duration_mins: u32,
    #[serde(rename = "resetsAt")]
    resets_at: i64,
}

/// Parses a raw `account/rateLimits/read` JSON-RPC result body into the
/// normalized `UsageSnapshot`. Pure function — where the JSON came from
/// (fixture today, a live process later) is the caller's concern.
pub fn parse_rate_limits(raw_json: &str) -> Result<UsageSnapshot, ProviderError> {
    let parsed: RawResponse = serde_json::from_str(raw_json)
        .map_err(|_| ProviderError("could not parse rate-limits response".to_string()))?;
    let limits = parsed.rate_limits;

    let mut note = "fixture data, not live".to_string();
    if let Some(plan) = &limits.plan_type {
        note.push_str(&format!("; plan: {plan}"));
    }
    if limits.ordinary_usage_allowed == Some(false) {
        note.push_str("; ordinary usage not allowed");
    }

    let mut counts = Vec::new();
    if let Some(credit) = limits.rate_limit_reset_credit {
        counts.push(UsageCount {
            label: "rate_limit_reset_credit".to_string(),
            used: 0,
            limit: credit,
        });
    }

    Ok(UsageSnapshot {
        provider: "codex".to_string(),
        status: ProviderStatus::Pending,
        session_usage_percent: Some(limits.primary.used_percent),
        session_reset_label: Some(format!(
            "resets at epoch {} ({}m window)",
            limits.primary.resets_at, limits.primary.window_duration_mins
        )),
        weekly_usage_percent: Some(limits.secondary.used_percent),
        weekly_reset_label: Some(format!(
            "resets at epoch {} ({}m window)",
            limits.secondary.resets_at, limits.secondary.window_duration_mins
        )),
        counts,
        note: Some(note),
    })
}

pub struct CodexProvider;

impl UsageProvider for CodexProvider {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn fetch_usage(&self) -> Result<UsageSnapshot, ProviderError> {
        // Off by default. Set ALBERT_LIVE_CODEX=1 (`make run-live`) to
        // spawn the real app-server instead of reading the fixture.
        if std::env::var("ALBERT_LIVE_CODEX").as_deref() == Ok("1") {
            return Ok(match crate::process::call_codex_rate_limits() {
                Ok(raw) => match parse_rate_limits(&raw) {
                    Ok(mut snapshot) => {
                        snapshot.status = ProviderStatus::Available;
                        snapshot.note = Some("live app-server response".to_string());
                        snapshot
                    }
                    Err(_) => unsupported("live response did not match the expected shape"),
                },
                Err(reason) => unsupported(&reason),
            });
        }

        parse_rate_limits(FIXTURE)
    }
}

fn unsupported(reason: &str) -> UsageSnapshot {
    UsageSnapshot {
        provider: "codex".to_string(),
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
    fn parses_allowlisted_fields_from_fixture() {
        let snapshot = parse_rate_limits(FIXTURE).unwrap();
        assert_eq!(snapshot.provider, "codex");
        assert_eq!(snapshot.status, ProviderStatus::Pending);
        assert_eq!(snapshot.session_usage_percent, Some(45.2));
        assert_eq!(snapshot.weekly_usage_percent, Some(12.7));
    }

    #[test]
    fn maps_rate_limit_reset_credit_as_a_count() {
        let snapshot = parse_rate_limits(FIXTURE).unwrap();
        assert_eq!(snapshot.counts.len(), 1);
        assert_eq!(snapshot.counts[0].label, "rate_limit_reset_credit");
        assert_eq!(snapshot.counts[0].limit, 2);
    }

    #[test]
    fn note_mentions_fixture_and_plan_never_raw_fields() {
        let snapshot = parse_rate_limits(FIXTURE).unwrap();
        let note = snapshot.note.unwrap();
        assert!(note.contains("fixture data, not live"));
        assert!(note.contains("plan: plus"));
        assert!(!note.contains("acct_"));
        assert!(!note.contains("credentials"));
    }

    #[test]
    fn malformed_json_returns_sanitized_error() {
        let err = parse_rate_limits("{not json").unwrap_err();
        assert_eq!(err.0, "could not parse rate-limits response");
    }

    #[test]
    fn codex_provider_identity() {
        let provider = CodexProvider;
        assert_eq!(provider.id(), "codex");
        assert_eq!(provider.display_name(), "Codex");
    }
}
