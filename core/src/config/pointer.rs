//! RFC 6901 JSON Pointer resolution against a `serde_json::Value`, and
//! the value-conversion rules the schema's `as` fields use (percent /
//! fraction / remainingFraction, iso8601 / epochSeconds / epochMillis,
//! seconds / minutes / windowName).

use serde_json::Value;

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

/// Converts a raw JSON value per `resetsAt.as` into an ISO-8601-ish
/// display string. `epochSeconds`/`epochMillis` are rendered as a
/// plain UTC-offset-free label (`epoch <n>s`) -- full calendar
/// formatting would need a date/time dependency this project doesn't
/// carry yet; `iso8601` values are already human-readable and passed
/// through as-is.
pub fn resets_at_label(value: &Value, as_: &crate::config::schema::ResetAs) -> Option<String> {
    use crate::config::schema::ResetAs;
    match as_ {
        ResetAs::Iso8601 => value.as_str().map(|s| s.to_string()),
        ResetAs::EpochSeconds => value.as_i64().map(|n| format!("epoch {n}s")),
        ResetAs::EpochMillis => value.as_i64().map(|n| format!("epoch {n}ms")),
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
    fn iso8601_passes_through() {
        assert_eq!(
            resets_at_label(&json!("2026-10-05T12:16:59Z"), &ResetAs::Iso8601),
            Some("2026-10-05T12:16:59Z".to_string())
        );
    }

    #[test]
    fn epoch_seconds_labeled() {
        assert_eq!(
            resets_at_label(&json!(1790680997), &ResetAs::EpochSeconds),
            Some("epoch 1790680997s".to_string())
        );
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
