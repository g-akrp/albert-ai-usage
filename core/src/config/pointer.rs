//! RFC 6901 JSON Pointer resolution against a `serde_json::Value`, and
//! the value-conversion rules the schema's `as` fields use (percent /
//! fraction / remainingFraction, iso8601 / epochSeconds / epochMillis,
//! seconds / minutes / windowName).

use chrono::{DateTime, Local, TimeZone};
use serde_json::Value;

/// Renders a UTC instant in the device's own local timezone (whatever
/// the OS clock is set to), e.g. `"Sep 29, 2026 6:23 PM"` -- readable,
/// not a raw ISO/epoch value.
fn format_local(dt: DateTime<Local>) -> String {
    dt.format("%b %-d, %Y %-I:%M %p").to_string()
}

/// Resolves a JSON Pointer like `/rate_limits/five_hour` against
/// `root`. An empty string resolves to `root` itself (the whole
/// document, per the schema's `root` default). Returns `None` when
/// any segment is missing -- callers treat that as "not present",
/// never a parse error.
pub fn resolve<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(root);
    }
    let mut current = root;
    for raw_segment in pointer.trim_start_matches('/').split('/') {
        let segment = raw_segment.replace("~1", "/").replace("~0", "~");
        current = match current {
            Value::Object(map) => map.get(&segment)?,
            Value::Array(arr) => arr.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// Converts a raw JSON number/bool per `used.as`: `percent` (100 is
/// full), `fraction` (1 is full), or `remainingFraction` (0..1 left,
/// inverted to a used percentage). Always returns a 0..=100-ish
/// percent (can exceed 100 for overage, per the schema's "usage past
/// the allowance is kept as overage").
pub fn used_as_percent(value: &Value, as_: &crate::config::schema::UsedAs) -> Option<f32> {
    use crate::config::schema::UsedAs;
    let n = value.as_f64()? as f32;
    Some(match as_ {
        UsedAs::Percent => n,
        UsedAs::Fraction => n * 100.0,
        UsedAs::RemainingFraction => (1.0 - n) * 100.0,
        UsedAs::RemainingPercent => 100.0 - n,
    })
}

/// Converts a raw JSON value per `resetsAt.as` into a readable,
/// device-local-timezone display string (e.g. `"Oct 1, 2026 12:00 AM"`),
/// not a raw ISO/epoch value. Falls back to the raw value as a string
/// only if it fails to parse -- never errors, since a bad/unexpected
/// reset value shouldn't break the rest of the report.
pub fn resets_at_label(value: &Value, as_: &crate::config::schema::ResetAs) -> Option<String> {
    use crate::config::schema::ResetAs;
    match as_ {
        ResetAs::Iso8601 => {
            let raw = value.as_str()?;
            let parsed =
                DateTime::parse_from_rfc3339(raw).map(|dt| format_local(dt.with_timezone(&Local)));
            Some(parsed.unwrap_or_else(|_| raw.to_string()))
        }
        ResetAs::EpochSeconds => {
            let secs = value.as_i64()?;
            Local
                .timestamp_opt(secs, 0)
                .single()
                .map(format_local)
                .or(Some(format!("epoch {secs}s")))
        }
        ResetAs::EpochMillis => {
            let millis = value.as_i64()?;
            Local
                .timestamp_millis_opt(millis)
                .single()
                .map(format_local)
                .or(Some(format!("epoch {millis}ms")))
        }
    }
}

/// Converts a `duration` spec into seconds. `windowName` recognizes
/// the fixed vocabulary the schema documents.
pub fn duration_seconds(root: &Value, spec: &crate::config::schema::DurationSpec) -> Option<u64> {
    use crate::config::schema::{DurationAs, DurationSpec};
    match spec {
        DurationSpec::Seconds { seconds } => Some(*seconds),
        DurationSpec::FromPath { path, as_ } => {
            let value = resolve(root, path)?;
            match as_ {
                DurationAs::Seconds => value.as_u64(),
                DurationAs::Minutes => value.as_u64().map(|m| m * 60),
                DurationAs::WindowName => {
                    let name = value.as_str()?;
                    Some(match name {
                        "five_hour" | "5h" => 5 * 3600,
                        "daily" | "day" => 24 * 3600,
                        "weekly" | "week" | "7d" => 7 * 24 * 3600,
                        _ => return None,
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{DurationAs, DurationSpec, ResetAs, UsedAs};
    use serde_json::json;

    #[test]
    fn resolves_nested_pointer() {
        let root = json!({"rate_limits": {"five_hour": {"utilization": 14}}});
        assert_eq!(
            resolve(&root, "/rate_limits/five_hour/utilization"),
            Some(&json!(14))
        );
    }

    #[test]
    fn empty_pointer_resolves_to_root() {
        let root = json!({"a": 1});
        assert_eq!(resolve(&root, ""), Some(&root));
    }

    #[test]
    fn missing_segment_is_none_not_a_panic() {
        let root = json!({"a": 1});
        assert_eq!(resolve(&root, "/b/c"), None);
    }

    #[test]
    fn resolves_array_index() {
        let root = json!({"groups": [{"name": "first"}, {"name": "second"}]});
        assert_eq!(resolve(&root, "/groups/1/name"), Some(&json!("second")));
    }

    #[test]
    fn percent_passthrough() {
        assert_eq!(used_as_percent(&json!(14), &UsedAs::Percent), Some(14.0));
    }

    #[test]
    fn fraction_converts_to_percent() {
        assert_eq!(used_as_percent(&json!(0.29), &UsedAs::Fraction), Some(29.0));
    }

    #[test]
    fn remaining_fraction_inverts_to_used_percent() {
        let pct = used_as_percent(&json!(0.9518590569496155), &UsedAs::RemainingFraction).unwrap();
        assert!((pct - 4.814).abs() < 0.01);
    }

    #[test]
    fn remaining_percent_inverts_to_used_percent() {
        assert_eq!(
            used_as_percent(&json!(40.3), &UsedAs::RemainingPercent),
            Some(59.7)
        );
    }

    #[test]
    fn iso8601_renders_readable_local_datetime() {
        let label = resets_at_label(&json!("2026-10-05T12:16:59Z"), &ResetAs::Iso8601).unwrap();
        // Device-local timezone varies by machine, so assert on the
        // readable shape (month name, year, 12-hour clock), not an
        // exact string.
        assert!(label.contains("2026"));
        assert!(label.contains("Oct"));
        assert!(label.contains("AM") || label.contains("PM"));
        assert!(!label.contains('T')); // not the raw ISO string
    }

    #[test]
    fn iso8601_falls_back_to_raw_on_unparseable_input() {
        assert_eq!(
            resets_at_label(&json!("not-a-date"), &ResetAs::Iso8601),
            Some("not-a-date".to_string())
        );
    }

    #[test]
    fn epoch_seconds_renders_readable_local_datetime() {
        let label = resets_at_label(&json!(1790680997), &ResetAs::EpochSeconds).unwrap();
        assert!(label.contains("2026"));
        assert!(label.contains("Sep"));
        assert!(label.contains("AM") || label.contains("PM"));
    }

    #[test]
    fn duration_from_window_name() {
        let root = json!({"window": "weekly"});
        let spec = DurationSpec::FromPath {
            path: "/window".to_string(),
            as_: DurationAs::WindowName,
        };
        assert_eq!(duration_seconds(&root, &spec), Some(7 * 24 * 3600));
    }

    #[test]
    fn duration_from_minutes() {
        let root = json!({"windowDurationMins": 300});
        let spec = DurationSpec::FromPath {
            path: "/windowDurationMins".to_string(),
            as_: DurationAs::Minutes,
        };
        assert_eq!(duration_seconds(&root, &spec), Some(18000));
    }
}
