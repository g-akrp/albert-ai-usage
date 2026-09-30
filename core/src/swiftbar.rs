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

/// A genuinely static, roughly square two-row icon (short label /
/// percent, both visible at once) via SwiftBar's `image=` param --
/// not two cycling text lines. SwiftBar's own text titles can't stack
/// two rows (confirmed against its plugin API docs); rendering a
/// small bitmap is the real way to get this. `icon_label` (not the
/// full display name -- too long to stay square) is drawn with a tiny
/// hand-authored pixel font in `icon.rs`.
fn push_icon_header(out: &mut String, name: &str, icon_label: &str, pct: f32) {
    let pct_text = format!("{pct:.0}%");
    let color = match percent_color(pct) {
        "red" => [200, 0, 0],
        "orange" => [200, 120, 0],
        _ => [0, 130, 0],
    };
    let png = crate::icon::render_two_line_png(icon_label, &pct_text, color, 2);
    let b64 = crate::icon::to_base64(&png);
    out.push_str(&format!("{name} {pct_text} | image={b64}\n"));
}

/// One provider's run: its config id, full display name (dropdown),
/// short icon label (menu-bar icon top row), and result.
pub struct ProviderRun {
    pub id: String,
    pub name: String,
    pub icon_label: String,
    pub result: Result<Report, String>,
}

/// Renders the full SwiftBar plugin text. `binary_path` must be this
/// binary's own absolute path (SwiftBar's `bash=` action needs an
/// absolute path to re-invoke it with `--pin <id>`).
pub fn render(binary_path: &str, pinned: Option<&str>, runs: &[ProviderRun]) -> String {
    let mut out = String::new();

    render_header(&mut out, pinned, runs);
    out.push_str("---\n");
    for run in runs {
        render_provider(&mut out, binary_path, run, pinned == Some(run.id.as_str()));
    }

    out
}

fn render_header(out: &mut String, pinned: Option<&str>, runs: &[ProviderRun]) {
    if let Some(pinned_id) = pinned {
        if let Some(run) = runs.iter().find(|r| r.id == pinned_id) {
            if let Ok(report) = &run.result {
                match max_percent(report) {
                    Some(pct) => {
                        push_icon_header(out, &run.name, &run.icon_label, pct);
                        return;
                    }
                    None => {
                        out.push_str(&format!("{}\n", run.name));
                        return;
                    }
                }
            }
        }
        // Pinned id doesn't match any run, or that run errored (stale
        // pin, e.g. a removed provider config) -- fall through.
    }

    // No valid pin: one icon line per provider (SwiftBar cycles
    // between full static icons rather than between name-only/
    // percent-only text halves).
    let mut any = false;
    for run in runs {
        if let Ok(report) = &run.result {
            if let Some(pct) = max_percent(report) {
                push_icon_header(out, &run.name, &run.icon_label, pct);
                any = true;
            }
        }
    }
    if !any {
        out.push_str("Agent Usage\n");
    }
}

fn render_provider(out: &mut String, binary_path: &str, run: &ProviderRun, is_pinned: bool) {
    let mark = if is_pinned { "\u{2605}" } else { "\u{2606}" }; // ★ / ☆
    let name = &run.name;
    match &run.result {
        Ok(report) => {
            let plan = report
                .plan
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            let id = &run.id;
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

    fn run(id: &str, name: &str, icon_label: &str, result: Result<Report, String>) -> ProviderRun {
        ProviderRun {
            id: id.to_string(),
            name: name.to_string(),
            icon_label: icon_label.to_string(),
            result,
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
            run("codex", "Codex", "CDX", Ok(sample_report(45.0))),
            run("copilot", "GitHub Copilot", "GHC", Ok(sample_report(10.0))),
        ];
        let out = render("/usr/local/bin/albert-usage", Some("codex"), &runs);
        let header: Vec<&str> = out.lines().take_while(|l| *l != "---").collect();
        assert_eq!(header.len(), 1); // one static icon line, not cycling
        assert!(header[0].starts_with("Codex 45%"));
        assert!(header[0].contains("image="));
    }

    #[test]
    fn header_cycles_all_providers_when_nothing_pinned() {
        let runs = vec![
            run("codex", "Codex", "CDX", Ok(sample_report(45.0))),
            run("copilot", "GitHub Copilot", "GHC", Ok(sample_report(10.0))),
        ];
        let out = render("/usr/local/bin/albert-usage", None, &runs);
        let header: Vec<&str> = out.lines().take_while(|l| *l != "---").collect();
        // One icon line per provider -- SwiftBar cycles between them.
        assert_eq!(header.len(), 2);
    }

    #[test]
    fn stale_pin_falls_back_to_cycling() {
        let runs = vec![run("codex", "Codex", "CDX", Ok(sample_report(45.0)))];
        let out = render(
            "/usr/local/bin/albert-usage",
            Some("removed-provider"),
            &runs,
        );
        let header: Vec<&str> = out.lines().take_while(|l| *l != "---").collect();
        assert_eq!(header.len(), 1);
        assert!(header[0].starts_with("Codex 45%"));
        assert!(header[0].contains("image="));
    }

    #[test]
    fn errored_pin_falls_back_to_cycling_not_blank() {
        let runs = vec![
            run("codex", "Codex", "CDX", Err("timed out".to_string())),
            run("copilot", "GitHub Copilot", "GHC", Ok(sample_report(10.0))),
        ];
        let out = render("/usr/local/bin/albert-usage", Some("codex"), &runs);
        let header: Vec<&str> = out.lines().take_while(|l| *l != "---").collect();
        assert_eq!(header.len(), 1);
        assert!(header[0].starts_with("GitHub Copilot 10%"));
    }

    #[test]
    fn provider_title_line_has_pin_action_with_its_own_id() {
        let runs = vec![run("codex", "Codex", "CDX", Ok(sample_report(45.0)))];
        let out = render("/usr/local/bin/albert-usage", None, &runs);
        assert!(out.contains("bash=/usr/local/bin/albert-usage param1=--pin param2=codex"));
        assert!(out.contains("terminal=false refresh=true"));
    }

    #[test]
    fn pinned_provider_shows_filled_star() {
        let runs = vec![run("codex", "Codex", "CDX", Ok(sample_report(45.0)))];
        let out = render("/usr/local/bin/albert-usage", Some("codex"), &runs);
        assert!(out.contains("\u{2605} Codex"));
    }

    #[test]
    fn error_row_shows_reason_no_pin_action() {
        let runs = vec![run("codex", "Codex", "CDX", Err("timed out".to_string()))];
        let out = render("/usr/local/bin/albert-usage", None, &runs);
        assert!(out.contains("Codex — Error: timed out"));
        assert!(!out.contains("bash="));
    }
}
