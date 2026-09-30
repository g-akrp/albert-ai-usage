//! Applies a config's `map` section to a captured JSON document,
//! producing a generic `Report` -- the provider-agnostic shape the
//! CLI prints. This is what makes providers data instead of code: a
//! new provider's `map` describes how to read its JSON, nothing here
//! is provider-specific.

use serde_json::Value;

use super::pointer::{duration_seconds, resets_at_label, resolve, used_as_percent};
use super::schema::{IdSpec, LabelSpec, Map, MeterSpec, Scope, WindowSpec};

#[derive(Debug, PartialEq)]
pub struct Report {
    pub plan: Option<String>,
    pub available: Option<bool>,
    pub access: Option<bool>,
    pub meters: Vec<MeterReport>,
}

#[derive(Debug, PartialEq)]
pub struct MeterReport {
    pub id: String,
    pub label: String,
    pub windows: Vec<WindowReport>,
}

#[derive(Debug, PartialEq)]
pub struct WindowReport {
    pub id: String,
    pub label: Option<String>,
    pub used_percent: Option<f32>,
    pub resets_at: Option<String>,
    pub duration_seconds: Option<u64>,
}

/// Every scoped value alongside its entry key, when the scope was
/// `eachEntry` (needed for `{"entryKey": true}` id/label specs).
fn scoped_values<'a>(current: &'a Value, scope: &Scope) -> Vec<(Option<String>, &'a Value)> {
    if let Some(path) = &scope.select {
        return resolve(current, path)
            .map(|v| vec![(None, v)])
            .unwrap_or_default();
    }
    if let Some(path) = &scope.each {
        return resolve(current, path)
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|v| (None, v)).collect())
            .unwrap_or_default();
    }
    if let Some(path) = &scope.each_entry {
        return resolve(current, path)
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (Some(k.clone()), v)).collect())
            .unwrap_or_default();
    }
    // No scope: read the current value as-is.
    vec![(None, current)]
}

fn resolve_id(spec: &IdSpec, value: &Value, entry_key: &Option<String>) -> Option<String> {
    match spec {
        IdSpec::Literal(s) => Some(s.clone()),
        IdSpec::Path { path } => resolve(value, path).and_then(value_as_id_string),
        IdSpec::EntryKey { entry_key: true } => entry_key.clone(),
        IdSpec::EntryKey { entry_key: false } => None,
    }
}

fn value_as_id_string(v: &Value) -> Option<String> {
    v.as_str()
        .map(|s| s.to_string())
        .or_else(|| v.as_i64().map(|n| n.to_string()))
}

fn translate_key(key: &str) -> String {
    match key {
        "plan" => "Plan",
        "session" => "Session",
        "weekly" => "Weekly",
        "monthly" => "Monthly",
        other => return other.to_string(),
    }
    .to_string()
}

fn resolve_label(spec: &LabelSpec, value: &Value) -> Option<String> {
    match spec {
        LabelSpec::Text { text } => Some(text.clone()),
        LabelSpec::Key { key } => Some(translate_key(key)),
        LabelSpec::PathFallback { path, fallback } => resolve(value, path)
            .and_then(value_as_id_string)
            .or_else(|| fallback.as_ref().and_then(|f| resolve_label(f, value))),
    }
}

fn map_window(spec: &WindowSpec, meter_value: &Value) -> Vec<WindowReport> {
    scoped_values(meter_value, &spec.scope)
        .into_iter()
        .filter_map(|(entry_key, window_value)| {
            let used_percent = spec
                .used
                .as_ref()
                .and_then(|u| resolve(window_value, &u.path))
                .zip(spec.used.as_ref())
                .and_then(|(v, u)| used_as_percent(v, &u.as_));
            let resets_at = spec
                .resets_at
                .as_ref()
                .and_then(|r| resolve(window_value, &r.path).zip(Some(r)))
                .and_then(|(v, r)| resets_at_label(v, &r.as_));

            // "A window with neither a usage value nor a reset time
            // is skipped."
            if used_percent.is_none() && resets_at.is_none() {
                return None;
            }

            let id = resolve_id(&spec.id, window_value, &entry_key)?;
            let label = spec
                .label
                .as_ref()
                .and_then(|l| resolve_label(l, window_value));
            let duration = spec
                .duration
                .as_ref()
                .and_then(|d| duration_seconds(window_value, d));

            Some(WindowReport {
                id,
                label,
                used_percent,
                resets_at,
                duration_seconds: duration,
            })
        })
        .collect()
}

fn map_meter(spec: &MeterSpec, root: &Value) -> Vec<MeterReport> {
    scoped_values(root, &spec.scope)
        .into_iter()
        .filter_map(|(entry_key, meter_value)| {
            let id = resolve_id(&spec.id, meter_value, &entry_key)?;
            let label = resolve_label(&spec.label, meter_value)?;
            let windows: Vec<WindowReport> = spec
                .windows
                .iter()
                .flat_map(|w| map_window(w, meter_value))
                .collect();
            if windows.is_empty() {
                // "so is a meter left without windows"
                return None;
            }
            Some(MeterReport { id, label, windows })
        })
        .collect()
}

/// Applies `map` to `captured` (the raw JSON the source produced).
pub fn apply_map(map: &Map, captured: &Value) -> Report {
    let root = map
        .root
        .as_deref()
        .and_then(|p| resolve(captured, p))
        .unwrap_or(captured);

    let plan = map.plan.as_ref().and_then(|p| {
        let raw = resolve(root, &p.path).and_then(value_as_id_string)?;
        Some(p.names.get(&raw).cloned().unwrap_or(raw))
    });
    let available = map
        .available
        .as_ref()
        .and_then(|a| resolve(root, &a.path))
        .and_then(|v| v.as_bool());
    let access = map
        .access
        .as_ref()
        .and_then(|a| resolve(root, &a.path))
        .and_then(|v| v.as_bool());

    let meters = map.meters.iter().flat_map(|m| map_meter(m, root)).collect();

    Report {
        plan,
        available,
        access,
        meters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::*;
    use serde_json::json;

    #[test]
    fn maps_a_single_current_value_meter_with_two_windows() {
        // Mirrors codex.json's shape: one meter read from the current
        // scope (no each/select), two windows under /primary,
        // /secondary.
        let captured = json!({
            "result": {
                "rateLimits": {
                    "limitId": "codex",
                    "primary": {"usedPercent": 45, "resetsAt": 1790680997, "windowDurationMins": 300},
                    "secondary": {"usedPercent": 12, "resetsAt": 1791075882, "windowDurationMins": 10080}
                }
            }
        });
        let map = Map {
            root: Some("/result".to_string()),
            available: None,
            plan: None,
            access: None,
            meters: vec![MeterSpec {
                scope: Scope {
                    select: Some("/rateLimits".to_string()),
                    each: None,
                    each_entry: None,
                },
                id: IdSpec::Path {
                    path: "/limitId".to_string(),
                },
                label: LabelSpec::Text {
                    text: "Codex".to_string(),
                },
                plan: None,
                limit_reached_reason: None,
                windows: vec![
                    WindowSpec {
                        scope: Scope {
                            select: Some("/primary".to_string()),
                            each: None,
                            each_entry: None,
                        },
                        id: IdSpec::Literal("primary".to_string()),
                        label: None,
                        used: Some(UsedSpec {
                            path: "/usedPercent".to_string(),
                            as_: UsedAs::Percent,
                        }),
                        resets_at: Some(AsSpec {
                            path: "/resetsAt".to_string(),
                            as_: ResetAs::EpochSeconds,
                        }),
                        duration: Some(DurationSpec::FromPath {
                            path: "/windowDurationMins".to_string(),
                            as_: DurationAs::Minutes,
                        }),
                    },
                    WindowSpec {
                        scope: Scope {
                            select: Some("/secondary".to_string()),
                            each: None,
                            each_entry: None,
                        },
                        id: IdSpec::Literal("secondary".to_string()),
                        label: None,
                        used: Some(UsedSpec {
                            path: "/usedPercent".to_string(),
                            as_: UsedAs::Percent,
                        }),
                        resets_at: Some(AsSpec {
                            path: "/resetsAt".to_string(),
                            as_: ResetAs::EpochSeconds,
                        }),
                        duration: Some(DurationSpec::FromPath {
                            path: "/windowDurationMins".to_string(),
                            as_: DurationAs::Minutes,
                        }),
                    },
                ],
            }],
        };

        let report = apply_map(&map, &captured);
        assert_eq!(report.meters.len(), 1);
        assert_eq!(report.meters[0].id, "codex");
        assert_eq!(report.meters[0].windows.len(), 2);
        assert_eq!(report.meters[0].windows[0].used_percent, Some(45.0));
        assert_eq!(
            report.meters[0].windows[1].duration_seconds,
            Some(10080 * 60)
        );
    }

    #[test]
    fn each_entry_scope_uses_object_key_as_id() {
        let captured = json!({
            "rateLimitsByLimitId": {
                "codex": {"primary": {"usedPercent": 50}}
            }
        });
        let map = Map {
            root: None,
            available: None,
            plan: None,
            access: None,
            meters: vec![MeterSpec {
                scope: Scope {
                    select: None,
                    each: None,
                    each_entry: Some("/rateLimitsByLimitId".to_string()),
                },
                id: IdSpec::EntryKey { entry_key: true },
                label: label_stub(),
                plan: None,
                limit_reached_reason: None,
                windows: vec![WindowSpec {
                    scope: Scope {
                        select: Some("/primary".to_string()),
                        each: None,
                        each_entry: None,
                    },
                    id: IdSpec::Literal("primary".to_string()),
                    label: None,
                    used: Some(UsedSpec {
                        path: "/usedPercent".to_string(),
                        as_: UsedAs::Percent,
                    }),
                    resets_at: None,
                    duration: None,
                }],
            }],
        };
        let report = apply_map(&map, &captured);
        assert_eq!(report.meters[0].id, "codex");
    }

    fn label_stub() -> LabelSpec {
        LabelSpec::Text {
            text: "x".to_string(),
        }
    }

    #[test]
    fn window_with_neither_used_nor_reset_is_skipped() {
        let captured = json!({"w": {}});
        let spec = WindowSpec {
            scope: Scope {
                select: Some("/w".to_string()),
                each: None,
                each_entry: None,
            },
            id: IdSpec::Literal("w".to_string()),
            label: None,
            used: Some(UsedSpec {
                path: "/missing".to_string(),
                as_: UsedAs::Percent,
            }),
            resets_at: None,
            duration: None,
        };
        assert!(map_window(&spec, &captured).is_empty());
    }

    #[test]
    fn plan_name_translated() {
        let captured = json!({"rateLimits": {"planType": "prolite"}});
        let map = Map {
            root: None,
            available: None,
            plan: Some(PlanRef {
                path: "/rateLimits/planType".to_string(),
                names: [("prolite".to_string(), "Pro Lite".to_string())]
                    .into_iter()
                    .collect(),
            }),
            access: None,
            meters: vec![],
        };
        let report = apply_map(&map, &captured);
        assert_eq!(report.plan, Some("Pro Lite".to_string()));
    }
}
