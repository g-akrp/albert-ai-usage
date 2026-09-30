pub mod config;
pub mod icon;
pub mod swiftbar;

use config::mapping::Report;

/// Formats one provider's report as a simple human-readable block.
/// Pure function, no I/O — unit-testable independent of the CLI.
pub fn format_report(id: &str, name: &str, result: &Result<Report, String>) -> String {
    match result {
        Ok(report) => {
            let mut lines = vec![format!("{name} ({id})")];
            if let Some(plan) = &report.plan {
                lines.push(format!("  plan: {plan}"));
            }
            if report.available == Some(false) {
                lines.push("  Plan limits don't apply to this account".to_string());
            }
            if report.access == Some(false) {
                lines.push("  Usage limit reached".to_string());
            }
            if report.meters.is_empty() {
                lines.push("  (no usage data)".to_string());
            }
            for meter in &report.meters {
                lines.push(format!("  {}:", meter.label));
                for window in &meter.windows {
                    let label = window.label.as_deref().unwrap_or(&window.id);
                    let percent = window
                        .used_percent
                        .map(|p| format!("{p:.0}%"))
                        .unwrap_or_else(|| "?".to_string());
                    let reset = window
                        .resets_at
                        .as_deref()
                        .map(|r| format!(" (resets {r})"))
                        .unwrap_or_default();
                    lines.push(format!("    {label}: {percent}{reset}"));
                }
            }
            lines.join("\n")
        }
        Err(reason) => format!("{name} ({id}) — Error: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::mapping::{MeterReport, WindowReport};

    #[test]
    fn formats_meters_and_windows() {
        let report = Report {
            plan: Some("Plus".to_string()),
            available: None,
            access: None,
            meters: vec![MeterReport {
                id: "codex".to_string(),
                label: "Codex".to_string(),
                windows: vec![WindowReport {
                    id: "primary".to_string(),
                    label: Some("Session".to_string()),
                    used_percent: Some(45.0),
                    resets_at: Some("Sep 29, 2026 6:23 PM".to_string()),
                    duration_seconds: Some(18000),
                }],
            }],
        };
        let out = format_report("codex", "Codex", &Ok(report));
        assert!(out.contains("plan: Plus"));
        assert!(out.contains("Session: 45%"));
        assert!(out.contains("resets Sep 29, 2026 6:23 PM"));
    }

    #[test]
    fn formats_error() {
        let out = format_report("codex", "Codex", &Err("timed out".to_string()));
        assert!(out.contains("Error: timed out"));
    }

    #[test]
    fn access_false_shows_limit_reached() {
        let report = Report {
            plan: None,
            available: None,
            access: Some(false),
            meters: vec![],
        };
        let out = format_report("codex", "Codex", &Ok(report));
        assert!(out.contains("Usage limit reached"));
    }
}
