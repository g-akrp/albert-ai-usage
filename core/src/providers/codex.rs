//! Codex adapter. Source: local `codex app-server`, stdio JSON-RPC v2,
//! method `account/rateLimits/read` after an `initialize` handshake.
//! Field shapes confirmed against the authoritative protocol schema
//! (`codex app-server generate-json-schema --experimental`), not just
//! research. V0.1.0 default: reads a sanitized fixture, no live call.
//! Live capture is a separate, opt-in step (`ALBERT_LIVE_CODEX=1`,
//! `make run-live`, see `crate::process`).
//!
//! Allowlist only: rateLimits.{primary,secondary}.{usedPercent,
//! windowDurationMins,resetsAt}, rateLimits.planType,
//! ordinaryUsageAllowed, rateLimitResetCredits.availableCount.
//! Everything else in the real response (accountId, credentials,
//! rateLimitUpsell, rateLimitsByLimitId, individualLimit, and any
//! unknown field) is dropped by construction — the deserialize target
//! below has no field for them, so serde ignores them; they never
//! reach `UsageSnapshot`.

use serde::Deserialize;

use crate::provider::{ProviderError, ProviderStatus, UsageCount, UsageProvider, UsageSnapshot};

const FIXTURE: &str = include_str!("../../tests/fixtures/codex_rate_limits.json");

#[derive(Debug, Deserialize)]
struct RawResponse {
    #[serde(rename = "rateLimits")]
    rate_limits: RawRateLimits,
    #[serde(rename = "ordinaryUsageAllowed")]
    ordinary_usage_allowed: Option<bool>,
    #[serde(rename = "rateLimitResetCredits")]
    rate_limit_reset_credits: Option<RawResetCredits>,
}

#[derive(Debug, Deserialize)]
struct RawRateLimits {
    primary: Option<RawWindow>,
    secondary: Option<RawWindow>,
    #[serde(rename = "planType")]
    plan_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawWindow {
    #[serde(rename = "usedPercent")]
    used_percent: i32,
    #[serde(rename = "windowDurationMins")]
    window_duration_mins: Option<i64>,
    #[serde(rename = "resetsAt")]
    resets_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RawResetCredits {
    #[serde(rename = "availableCount")]
    available_count: i64,
}

fn window_label(window: &RawWindow) -> String {
    let reset = window
        .resets_at
        .map(|epoch| format!("resets at epoch {epoch}"))
        .unwrap_or_else(|| "reset time unknown".to_string());
    let duration = window
        .window_duration_mins
        .map(|m| format!(" ({m}m window)"))
        .unwrap_or_default();
    format!("{reset}{duration}")
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
    if parsed.ordinary_usage_allowed == Some(false) {
        note.push_str("; ordinary usage not allowed");
    }

    let mut counts = Vec::new();
    if let Some(credits) = parsed.rate_limit_reset_credits {
        counts.push(UsageCount {
            label: "rate_limit_reset_credits_available".to_string(),
            used: 0,
            limit: credits.available_count.max(0) as u32,
        });
    }

    Ok(UsageSnapshot {
        provider: "codex".to_string(),
        status: ProviderStatus::Pending,
        session_usage_percent: limits.primary.as_ref().map(|w| w.used_percent as f32),
        session_reset_label: limits.primary.as_ref().map(window_label),
        weekly_usage_percent: limits.secondary.as_ref().map(|w| w.used_percent as f32),
        weekly_reset_label: limits.secondary.as_ref().map(window_label),
        counts,
        note: Some(note),
    })
}

pub struct CodexProvider;

impl UsageProvider for CodexProvider {
    fn id(&self) -> String {
        "codex".to_string()
    }

    fn display_name(&self) -> String {
        "Codex".to_string()
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
        assert_eq!(snapshot.session_usage_percent, Some(45.0));
        assert_eq!(snapshot.weekly_usage_percent, Some(12.0));
    }

    #[test]
    fn maps_reset_credits_available_count_as_a_count() {
        let snapshot = parse_rate_limits(FIXTURE).unwrap();
        assert_eq!(snapshot.counts.len(), 1);
        assert_eq!(
            snapshot.counts[0].label,
            "rate_limit_reset_credits_available"
        );
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
        assert!(!note.contains("should_never_appear"));
    }

    #[test]
    fn missing_window_yields_none_not_an_error() {
        let json = r#"{"rateLimits":{"primary":null,"secondary":null,"planType":null}}"#;
        let snapshot = parse_rate_limits(json).unwrap();
        assert_eq!(snapshot.session_usage_percent, None);
        assert_eq!(snapshot.weekly_usage_percent, None);
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
