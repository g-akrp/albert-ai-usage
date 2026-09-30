//! Renders provider reports as a SwiftBar plugin (menu bar text +
//! dropdown), and the "pinned provider" state SwiftBar's click
//! actions read/write. Verified against SwiftBar's own plugin API
//! docs (github.com/swiftbar/SwiftBar#writing-plugins), not guessed:
//! header is everything before the first `---`, each subsequent line
//! is `Title | param=value ...`, and `bash=<path> param1=... terminal=
//! false refresh=true` runs this binary again on click.
//!
//! One clickable action per provider: clicking its title line pins it
//! (`--pin <id>`), which changes what the compact menu-bar line shows
//! on the next refresh. Nothing else is clickable -- window detail
//! lines are plain text.

use crate::config::mapping::Report;

/// The most urgent number to show compactly: the highest used-percent
/// across every window in the report (worst case is the most useful
/// at a glance), or `None` if the report has no numeric windows.
pub fn max_percent(report: &Report) -> Option<f32> {
    report
        .meters
        .iter()
        .flat_map(|m| &m.windows)
        .filter_map(|w| w.used_percent)
        .fold(None, |acc, p| Some(acc.map_or(p, |a: f32| a.max(p))))
}

fn percent_color(pct: f32) -> &'static str {
    if pct >= 90.0 {
        "red"
    } else if pct >= 70.0 {
        "orange"
    } else {
        "green"
    }
}

/// Two separate header lines -- name, then percent -- so SwiftBar
/// cycles them ("Codex" then "47%") instead of one long combined
/// line. Both lines are small (`size=9`) to read as a compact
/// two-line label rather than a normal-sized menu title.
fn push_two_line_header(out: &mut String, name: &str, pct: f32) {
    out.push_str(&format!("{name} | size=9\n"));
    out.push_str(&format!(
        "{pct:.0}% | size=9 color={}\n",
        percent_color(pct)
    ));
}

/// One provider's run: its config id, display name, and result.
pub type ProviderRun = (String, String, Result<Report, String>);

/// Renders the full SwiftBar plugin text. `binary_path` must be this
/// binary's own absolute path (SwiftBar's `bash=` action needs an
/// absolute path to re-invoke it with `--pin <id>`).
pub fn render(binary_path: &str, pinned: Option<&str>, runs: &[ProviderRun]) -> String {
    let mut out = String::new();

    render_header(&mut out, pinned, runs);
    out.push_str("---\n");
    for (id, name, result) in runs {
        render_provider(&mut out, binary_path, id, name, result, pinned == Some(id));
    }

    out
}

fn render_header(out: &mut String, pinned: Option<&str>, runs: &[ProviderRun]) {
    if let Some(pinned_id) = pinned {
        if let Some((_, name, Ok(report))) = runs.iter().find(|(id, _, _)| id == pinned_id) {
            match max_percent(report) {
                Some(pct) => {
                    push_two_line_header(out, name, pct);
                    return;
                }
                None => {
                    out.push_str(&format!("{name}\n"));
                    return;
                }
            }
        }
        // Pinned id doesn't match any run (stale pin, e.g. a removed
        // provider config) -- fall through to the cycling default.
    }

    // No valid pin: cycle every provider's name/percent as separate
    // header lines (SwiftBar cycles multiple header lines -- there's
    // no documented way to make two lines stack as one static label,
    // per SwiftBar's own plugin API docs; this is the closest native
    // behavior: "Codex" then "47%" alternate every couple seconds).
    let mut any = false;
    for (_, name, result) in runs {
        if let Ok(report) = result {
            if let Some(pct) = max_percent(report) {
                push_two_line_header(out, name, pct);
                any = true;
            }
        }
    }
    if !any {
        out.push_str("Agent Usage\n");
    }
}

fn render_provider(
    out: &mut String,
    binary_path: &str,
    id: &str,
    name: &str,
    result: &Result<Report, String>,
    is_pinned: bool,
) {
    let mark = if is_pinned { "\u{2605}" } else { "\u{2606}" }; // ★ / ☆
    match result {
        Ok(report) => {
            let plan = report
                .plan
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            out.push_str(&format!(
                "{mark} {name}{plan} | md=true bash={binary_path} param1=--pin param2={id} terminal=false refresh=true\n"
            ));
            if report.access == Some(false) {
                out.push_str("\u{a0}\u{a0}Usage limit reached | color=red\n");
            }
            for meter in &report.meters {
                for window in &meter.windows {
                    let label = window.label.as_deref().unwrap_or(&window.id);
                    let (text, color) = match window.used_percent {
                        Some(pct) => (format!("{pct:.0}%"), percent_color(pct)),
                        None => ("?".to_string(), "gray"),
                    };
                    let reset = window
                        .resets_at
                        .as_deref()
                        .map(|r| format!(" (resets {r})"))
                        .unwrap_or_default();
                    out.push_str(&format!(
                        "\u{a0}\u{a0}{label}: {text}{reset} | color={color}\n"
                    ));
                }
            }
        }
        Err(reason) => {
            out.push_str(&format!("{mark} {name} — Error: {reason} | color=red\n"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::mapping::{MeterReport, WindowReport};

    fn sample_report(percent: f32) -> Report {
        Report {
            plan: Some("Plus".to_string()),
            available: None,
            access: None,
            meters: vec![MeterReport {
                id: "codex".to_string(),
                label: "Codex".to_string(),
                windows: vec![WindowReport {
                    id: "primary".to_string(),
                    label: Some("Session".to_string()),
                    used_percent: Some(percent),
                    resets_at: Some("2026-10-01T00:00:00Z".to_string()),
                    duration_seconds: None,
                }],
            }],
        }
    }

    #[test]
    fn max_percent_picks_the_highest_window() {
        let mut report = sample_report(10.0);
        report.meters[0].windows.push(WindowReport {
            id: "secondary".to_string(),
            label: Some("Weekly".to_string()),
            used_percent: Some(80.0),
            resets_at: None,
            duration_seconds: None,
        });
        assert_eq!(max_percent(&report), Some(80.0));
    }

    #[test]
    fn header_shows_pinned_provider_when_valid() {
        let runs = vec![
            (
                "codex".to_string(),
                "Codex".to_string(),
                Ok(sample_report(45.0)),
            ),
            (
                "copilot".to_string(),
                "GitHub Copilot".to_string(),
                Ok(sample_report(10.0)),
            ),
        ];
        let out = render("/usr/local/bin/albert-usage", Some("codex"), &runs);
        let header: Vec<&str> = out.lines().take_while(|l| *l != "---").collect();
        assert_eq!(header.len(), 2);
        assert!(header[0].starts_with("Codex"));
        assert!(header[1].starts_with("45%"));
    }

    #[test]
    fn header_cycles_all_providers_when_nothing_pinned() {
        let runs = vec![
            (
                "codex".to_string(),
                "Codex".to_string(),
                Ok(sample_report(45.0)),
            ),
            (
                "copilot".to_string(),
                "GitHub Copilot".to_string(),
                Ok(sample_report(10.0)),
            ),
        ];
        let out = render("/usr/local/bin/albert-usage", None, &runs);
        let header: Vec<&str> = out.lines().take_while(|l| *l != "---").collect();
        // Two providers, name + percent line each = 4 header lines.
        assert_eq!(header.len(), 4);
    }

    #[test]
    fn stale_pin_falls_back_to_cycling() {
        let runs = vec![(
            "codex".to_string(),
            "Codex".to_string(),
            Ok(sample_report(45.0)),
        )];
        let out = render(
            "/usr/local/bin/albert-usage",
            Some("removed-provider"),
            &runs,
        );
        let header: Vec<&str> = out.lines().take_while(|l| *l != "---").collect();
        assert_eq!(header, vec!["Codex | size=9", "45% | size=9 color=green"]);
    }

    #[test]
    fn provider_title_line_has_pin_action_with_its_own_id() {
        let runs = vec![(
            "codex".to_string(),
            "Codex".to_string(),
            Ok(sample_report(45.0)),
        )];
        let out = render("/usr/local/bin/albert-usage", None, &runs);
        assert!(out.contains("bash=/usr/local/bin/albert-usage param1=--pin param2=codex"));
        assert!(out.contains("terminal=false refresh=true"));
    }

    #[test]
    fn pinned_provider_shows_filled_star() {
        let runs = vec![(
            "codex".to_string(),
            "Codex".to_string(),
            Ok(sample_report(45.0)),
        )];
        let out = render("/usr/local/bin/albert-usage", Some("codex"), &runs);
        assert!(out.contains("\u{2605} Codex"));
    }

    #[test]
    fn error_row_shows_reason_no_pin_action() {
        let runs = vec![(
            "codex".to_string(),
            "Codex".to_string(),
            Err("timed out".to_string()),
        )];
        let out = render("/usr/local/bin/albert-usage", None, &runs);
        assert!(out.contains("Codex — Error: timed out"));
        assert!(!out.contains("bash="));
    }
}
