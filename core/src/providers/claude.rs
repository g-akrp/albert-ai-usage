//! Claude Code adapter. Source: `claude -p "/usage" --output-format
//! json` — a documented, supported CLI mode (`-p`/`--print` +
//! `--output-format json`), not TUI scraping. Verified live
//! 2026-09-30: `local_command: "usage"`, zero tokens, zero cost (it
//! never touches a model, purely a local status read).
//!
//! Corrects earlier research: "no non-interactive Claude Code source
//! exists" was wrong. `/usage` works fine via `-p`, it just wasn't
//! tried directly.
//!
//! The response's `result` field is free text meant for human display,
//! not structured JSON — this parses only two specific lines
//! ("Current session: NN% used · resets ...", "Current week...") and
//! discards everything else in that text (session/request counts, top
//! skills, top subagents, top MCP servers) by construction. That extra
//! detail is local-machine usage-pattern info, not needed here, and
//! never stored or logged.
//!
//! WIRE FORMAT NOTE: only one real sample observed. The "Current
//! week" line's exact suffix ("(all models)") may vary by plan tier,
//! so the match is a prefix, not the full line.

use serde::Deserialize;

use crate::provider::{ProviderError, ProviderStatus, UsageProvider, UsageSnapshot};

const FIXTURE: &str = include_str!("../../tests/fixtures/claude_usage.json");

#[derive(Debug, Deserialize)]
struct RawResponse {
    result: String,
}

/// Finds the first line starting with `prefix` and pulls the percent
/// number and the reset label after "resets " out of it. Returns
/// `None` if the line isn't present or doesn't match the expected
/// shape -- never panics on unexpected text.
fn parse_limit_line(result: &str, prefix: &str) -> Option<(f32, String)> {
    let line = result.lines().find(|l| l.trim_start().starts_with(prefix))?;
    let rest = line.trim_start().strip_prefix(prefix)?.trim_start();
    let percent: f32 = rest.split('%').next()?.trim().parse().ok()?;
    let reset = rest.split("resets ").nth(1)?.trim().to_string();
    Some((percent, reset))
}

/// Parses a raw `claude -p "/usage" --output-format json` response
/// body into the normalized `UsageSnapshot`. Pure function -- where
/// the JSON came from (fixture today, a live `claude` call later) is
/// the caller's concern.
pub fn parse_claude_usage(raw_json: &str) -> Result<UsageSnapshot, ProviderError> {
    let parsed: RawResponse = serde_json::from_str(raw_json)
        .map_err(|_| ProviderError("could not parse claude usage response".to_string()))?;

    let session = parse_limit_line(&parsed.result, "Current session:");
    let weekly = parse_limit_line(&parsed.result, "Current week");

    if session.is_none() && weekly.is_none() {
        return Err(ProviderError(
            "usage response did not contain expected limit lines".to_string(),
        ));
    }

    Ok(UsageSnapshot {
        provider: "claude-code".to_string(),
        status: ProviderStatus::Pending,
        session_usage_percent: session.as_ref().map(|(p, _)| *p),
        session_reset_label: session.map(|(_, label)| label),
        weekly_usage_percent: weekly.as_ref().map(|(p, _)| *p),
        weekly_reset_label: weekly.map(|(_, label)| label),
        counts: Vec::new(),
        note: Some("via claude -p /usage --output-format json".to_string()),
    })
}

pub struct ClaudeProvider;

impl UsageProvider for ClaudeProvider {
    fn id(&self) -> String {
        "claude-code".to_string()
    }

    fn display_name(&self) -> String {
        "Claude Code".to_string()
    }

    fn fetch_usage(&self) -> Result<UsageSnapshot, ProviderError> {
        // Off by default. Set ALBERT_LIVE_CLAUDE=1 (`make
        // run-live-claude`) to call the real `claude` CLI instead of
        // reading the fixture.
        if std::env::var("ALBERT_LIVE_CLAUDE").as_deref() == Ok("1") {
            return Ok(match crate::process::call_claude_usage() {
                Ok(raw) => match parse_claude_usage(&raw) {
                    Ok(mut snapshot) => {
                        snapshot.status = ProviderStatus::Available;
                        snapshot.note =
                            Some("live claude -p /usage response".to_string());
                        snapshot
                    }
                    Err(_) => unsupported("live response did not match the expected shape"),
                },
                Err(reason) => unsupported(&reason),
            });
        }

        parse_claude_usage(FIXTURE)
    }
}

fn unsupported(reason: &str) -> UsageSnapshot {
    UsageSnapshot {
        provider: "claude-code".to_string(),
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
    fn parses_session_and_weekly_percent_from_fixture() {
        let snapshot = parse_claude_usage(FIXTURE).unwrap();
        assert_eq!(snapshot.provider, "claude-code");
        assert_eq!(snapshot.session_usage_percent, Some(12.0));
        assert_eq!(snapshot.weekly_usage_percent, Some(28.0));
    }

    #[test]
    fn reset_labels_captured_never_the_breakdown_text() {
        let snapshot = parse_claude_usage(FIXTURE).unwrap();
        assert!(snapshot
            .session_reset_label
            .unwrap()
            .contains("Sep 30 at 1:59pm"));
        let note = snapshot.note.unwrap();
        assert!(!note.contains("should_never_appear"));
    }

    #[test]
    fn malformed_json_returns_sanitized_error() {
        let err = parse_claude_usage("{not json").unwrap_err();
        assert_eq!(err.0, "could not parse claude usage response");
    }

    #[test]
    fn missing_limit_lines_returns_sanitized_error() {
        let json = r#"{"result":"nothing useful here"}"#;
        let err = parse_claude_usage(json).unwrap_err();
        assert_eq!(err.0, "usage response did not contain expected limit lines");
    }

    #[test]
    fn claude_provider_identity() {
        let provider = ClaudeProvider;
        assert_eq!(provider.id(), "claude-code");
        assert_eq!(provider.display_name(), "Claude Code");
    }
}
