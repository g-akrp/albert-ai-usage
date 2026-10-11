use crate::{json, relative_time};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Count {
    pub used: f64,
    pub limit: f64,
    pub unit: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub id: String,
    pub label: Option<String>,
    pub used: Option<f64>,
    pub resets_at: Option<i64>,
    pub duration: Option<i64>,
    pub count: Option<Count>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Meter {
    pub id: String,
    pub label: String,
    pub windows: Vec<Window>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub plan: Option<String>,
    pub available: Option<bool>,
    pub access: Option<bool>,
    pub meters: Vec<Meter>,
}
fn scopes<'a>(root: &'a Value, spec: &Value) -> Vec<(Option<String>, &'a Value)> {
    if let Some(pointer) = spec["select"].as_str() {
        return root
            .pointer(pointer)
            .map(|v| vec![(None, v)])
            .unwrap_or_default();
    }
    if let Some(pointer) = spec["each"].as_str() {
        return match root.pointer(pointer) {
            Some(Value::Array(a)) => a.iter().map(|v| (None, v)).collect(),
            _ => Vec::new(),
        };
    }
    if let Some(pointer) = spec["eachEntry"].as_str() {
        return match root.pointer(pointer) {
            Some(Value::Object(o)) => o.iter().map(|(k, v)| (Some(k.clone()), v)).collect(),
            _ => Vec::new(),
        };
    }
    vec![(None, root)]
}
fn label(spec: &Value, value: &Value, key: Option<&str>) -> Option<String> {
    if let Some(s) = spec.as_str() {
        return Some(s.into());
    }
    if let Some(s) = spec["text"].as_str() {
        return Some(s.into());
    }
    if let Some(s) = spec["key"].as_str() {
        return Some(
            match s {
                "session" => "Session",
                "weekly" => "Weekly",
                "plan" => "Plan",
                "fiveHour" => "Five Hour",
                s => s,
            }
            .into(),
        );
    }
    if spec["entryKey"].as_bool() == Some(true) {
        return key.map(str::to_string);
    }
    spec["path"]
        .as_str()
        .and_then(|p| json::string(value.pointer(p)))
        .or_else(|| spec.get("fallback").and_then(|f| label(f, value, key)))
}
fn duration(spec: &Value, value: &Value) -> Option<i64> {
    if let Some(s) = spec["seconds"].as_i64() {
        return Some(s);
    }
    let v = value.pointer(spec["path"].as_str()?);
    match spec["as"].as_str()? {
        "seconds" => json::number(v).map(|n| n as i64),
        "minutes" => json::number(v).map(|n| (n * 60.0) as i64),
        "windowName" => v?.as_str().and_then(|s| {
            let s = s.to_lowercase();
            if s.contains("five") || s.contains("5h") {
                Some(18000)
            } else if s.contains("week") || s.contains("7d") {
                Some(604800)
            } else {
                None
            }
        }),
        _ => None,
    }
}
impl Window {
    pub fn priority(&self) -> u8 {
        let text = format!("{} {}", self.id, self.label.as_deref().unwrap_or("")).to_lowercase();
        if self.duration == Some(18000) || text.contains("session") || text.contains("five") {
            0
        } else if self.duration == Some(604800) || text.contains("week") || text.contains("seven") {
            1
        } else if text.contains("premium") {
            2
        } else {
            3
        }
    }
    pub fn percent_text(&self, counted: bool) -> String {
        let mut s = self
            .used
            .map(|n| format!("{n:.0}%"))
            .unwrap_or_else(|| "--".into());
        if counted {
            if let Some(c) = &self.count {
                s.push_str(&format!(" · {:.0}/{:.0}", c.used, c.limit));
                if let Some(unit) = &c.unit {
                    s.push(' ');
                    s.push_str(unit);
                }
            }
        }
        s
    }
}
fn map_window(spec: &Value, meter: &Value, answer: &Value) -> Vec<Window> {
    scopes(meter, spec)
        .into_iter()
        .filter_map(|(key, value)| {
            if !json::matches(&spec["match"], value) {
                return None;
            }
            let used = spec.get("used").and_then(|s| {
                let n = json::number(value.pointer(s["path"].as_str()?))?;
                let n = match s["as"].as_str()? {
                    "percent" => n,
                    "fraction" => n * 100.0,
                    "remainingFraction" => (1.0 - n) * 100.0,
                    "remainingPercent" => 100.0 - n,
                    _ => return None,
                };
                Some(n.clamp(0.0, 100.0))
            });
            let resets_at = spec.get("resetsAt").and_then(|s| {
                let root = if s["from"].as_str() == Some("root") {
                    answer
                } else {
                    value
                };
                let v = root.pointer(s["path"].as_str()?);
                match s["as"].as_str()? {
                    "iso8601" => relative_time::parse_date(v?.as_str()?),
                    "epochSeconds" => json::number(v).map(|n| n as i64),
                    "epochMillis" => json::number(v).map(|n| (n / 1000.0) as i64),
                    _ => None,
                }
            });
            if used.is_none() && resets_at.is_none() {
                return None;
            }
            let id = label(&spec["id"], value, key.as_deref())?;
            let duration = spec.get("duration").and_then(|s| duration(s, value));
            let label = spec
                .get("label")
                .and_then(|s| label(s, value, key.as_deref()))
                .or_else(|| {
                    duration.map(|d| {
                        if d == 18000 {
                            "Session".into()
                        } else if d == 604800 {
                            "Weekly".into()
                        } else {
                            format!("{}h", d / 3600)
                        }
                    })
                });
            let count = spec.get("count").and_then(|s| {
                let limit = json::number(value.pointer(s["limit"]["path"].as_str()?))?;
                if limit == 0.0 {
                    return None;
                }
                let remaining = json::number(value.pointer(s["remaining"]["path"].as_str()?))?;
                let unit = s
                    .get("unit")
                    .filter(|u| json::matches(&u["match"], value))
                    .and_then(|u| u["text"].as_str())
                    .map(str::to_string);
                Some(Count {
                    used: limit - remaining,
                    limit,
                    unit,
                })
            });
            Some(Window {
                id,
                label,
                used,
                resets_at,
                duration,
                count,
            })
        })
        .collect()
}
impl Report {
    pub fn map(spec: &Value, data: &Value) -> Result<Self, String> {
        let root = if let Some(p) = spec["root"].as_str() {
            data.pointer(p)
                .ok_or("usage response missing mapped root")?
        } else {
            data
        };
        let plan = spec["plan"]["path"]
            .as_str()
            .and_then(|p| root.pointer(p))
            .and_then(Value::as_str)
            .map(|s| spec["plan"]["names"][s].as_str().unwrap_or(s).to_string());
        let available = spec["available"]["path"]
            .as_str()
            .and_then(|p| root.pointer(p))
            .and_then(Value::as_bool);
        let access = spec["access"]["path"]
            .as_str()
            .and_then(|p| root.pointer(p))
            .and_then(Value::as_bool);
        let mut meters = Vec::new();
        for m in spec["meters"].as_array().ok_or("missing meters")? {
            for (key, value) in scopes(root, m) {
                if !json::matches(&m["match"], value) {
                    continue;
                }
                let Some(id) = label(&m["id"], value, key.as_deref()) else {
                    continue;
                };
                let Some(name) = label(&m["label"], value, key.as_deref()) else {
                    continue;
                };
                let mut windows = Vec::new();
                for w in m["windows"].as_array().ok_or("missing windows")? {
                    for window in map_window(w, value, root) {
                        if !windows.iter().any(|w: &Window| w.id == window.id) {
                            windows.push(window);
                        }
                    }
                }
                if !windows.is_empty() && !meters.iter().any(|m: &Meter| m.id == id) {
                    meters.push(Meter {
                        id,
                        label: name,
                        windows,
                    });
                }
            }
        }
        Ok(Self {
            plan,
            available,
            access,
            meters,
        })
    }
    pub fn max_percent(&self) -> Option<f64> {
        self.meters
            .iter()
            .flat_map(|m| &m.windows)
            .filter_map(|w| w.used)
            .reduce(f64::max)
    }
    pub fn headline(&self) -> Option<f64> {
        let mut values = Vec::new();
        for m in &self.meters {
            for w in &m.windows {
                if let Some(p) = w.used {
                    let priority = if w.priority() == 3 && m.id.to_lowercase().contains("premium") {
                        2
                    } else {
                        w.priority()
                    };
                    values.push((priority, p));
                }
            }
        }
        let rank = values.iter().map(|v| v.0).min()?;
        values
            .iter()
            .filter(|v| v.0 == rank)
            .map(|v| v.1)
            .reduce(f64::max)
    }
    pub fn grouped(&self) -> bool {
        self.meters.len() > 1
            && !self
                .meters
                .iter()
                .all(|m| m.windows.len() == 1 && m.windows[0].label.is_none())
    }
}
