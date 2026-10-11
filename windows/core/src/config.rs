use crate::json;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Source {
    pub executable: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub timeout: u64,
    pub kind: String,
    pub max_output: usize,
    pub max_line: usize,
    pub max_total: usize,
    pub raw: Value,
}
#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub label: String,
    pub color: [u8; 3],
    pub interval: u64,
    pub source: Source,
    pub accounts: Option<Value>,
    pub mapping: Value,
}
fn text<'a>(v: &'a Value, key: &str, max: usize) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= max)
        .ok_or_else(|| format!("/{key}: expected a nonempty string up to {max} bytes"))
}
fn bound(v: &Value, key: &str, default: u64, min: u64, max: u64) -> Result<u64, String> {
    let n = if v.get(key).is_some() {
        v[key]
            .as_u64()
            .ok_or_else(|| format!("/{key}: expected an integer"))?
    } else {
        default
    };
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(format!("/{key}: must be {min} to {max}"))
    }
}
fn pointer(v: &Value) -> bool {
    v.as_str()
        .is_some_and(|s| s.is_empty() || s.starts_with('/'))
}
fn predicates(v: &Value) -> Result<(), String> {
    if v.is_null() {
        return Ok(());
    }
    let items = v
        .as_array()
        .filter(|a| a.len() <= 32)
        .ok_or("/match: expected up to 32 predicates")?;
    for p in items {
        let tests = ["equals", "notEquals", "exists"]
            .iter()
            .filter(|k| p.get(**k).is_some())
            .count();
        if !pointer(&p["path"]) || tests != 1 || p.get("exists").is_some_and(|b| !b.is_boolean()) {
            return Err("/match: invalid predicate".into());
        }
    }
    Ok(())
}
fn validate_spec(v: &Value, depth: usize) -> Result<(), String> {
    if depth > 16 {
        return Err("/map: nesting too deep".into());
    }
    if let Some(o) = v.as_object() {
        predicates(&v["match"])?;
        if ["select", "each", "eachEntry"]
            .iter()
            .filter(|k| o.contains_key(**k))
            .count()
            > 1
        {
            return Err("/map: select, each and eachEntry are exclusive".into());
        }
        for (key, value) in o {
            if [
                "path",
                "select",
                "each",
                "eachEntry",
                "root",
                "require",
                "error",
            ]
            .contains(&key.as_str())
                && !pointer(value)
            {
                return Err(format!("/{key}: expected a JSON pointer"));
            }
            if !["equals", "notEquals"].contains(&key.as_str()) {
                validate_spec(value, depth + 1)?;
            }
        }
    } else if let Some(a) = v.as_array() {
        for x in a {
            validate_spec(x, depth + 1)?;
        }
    }
    Ok(())
}
impl Source {
    pub fn parse(raw: &Value) -> Result<Self, String> {
        let kind = text(raw, "type", 16)?.to_string();
        if kind != "command" && kind != "stdio" {
            return Err("/source/type: expected command or stdio".into());
        }
        let executable = text(raw, "executable", 1024)?.to_string();
        let args = match raw.get("args") {
            None => Vec::new(),
            Some(v) => v
                .as_array()
                .filter(|a| a.len() <= 32)
                .ok_or("/args: expected up to 32 strings")?
                .iter()
                .map(|a| {
                    a.as_str()
                        .filter(|s| s.len() <= 4096 && !s.contains('\0'))
                        .map(str::to_string)
                        .ok_or_else(|| "/args: invalid string".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let mut env = BTreeMap::new();
        if let Some(value) = raw.get("env") {
            let o = value
                .as_object()
                .filter(|o| o.len() <= 16)
                .ok_or("/env: expected up to 16 variables")?;
            for (k, v) in o {
                if k.is_empty()
                    || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    || k.starts_with(|c: char| c.is_ascii_digit())
                    || ["GH_TOKEN", "GITHUB_TOKEN"].contains(&k.to_ascii_uppercase().as_str())
                {
                    return Err("/env: invalid or reserved variable name".into());
                }
                let val = v
                    .as_str()
                    .filter(|s| s.len() <= 1024 && !s.contains('\0'))
                    .ok_or("/env: invalid value")?;
                env.insert(k.clone(), val.into());
            }
        }
        let timeout = bound(raw, "timeoutSeconds", 30, 1, 120)?;
        let max_output = bound(raw, "maxOutputBytes", 1048576, 1024, 8388608)? as usize;
        let max_line = bound(raw, "maxLineBytes", 1048576, 1024, 8388608)? as usize;
        let max_total = bound(raw, "maxTotalBytes", 8388608, 1024, 33554432)? as usize;
        if kind == "stdio" {
            let steps = raw["steps"]
                .as_array()
                .filter(|s| !s.is_empty() && s.len() <= 32)
                .ok_or("/steps: expected 1 to 32 steps")?;
            for step in steps {
                if step.get("write").is_some() == step.get("await").is_some() {
                    return Err("/steps: exactly one write or await required".into());
                }
                if let Some(predicate) = step.get("await") {
                    validate_spec(predicate, 0)?;
                }
            }
            text(raw, "output", 64)?;
        }
        if let Some(expect) = raw.get("expect") {
            validate_spec(expect, 0)?;
        }
        Ok(Self {
            executable,
            args,
            env,
            timeout,
            kind,
            max_output,
            max_line,
            max_total,
            raw: raw.clone(),
        })
    }
    pub fn account(&self, account: &str) -> Self {
        let mut s = self.clone();
        s.args = s
            .args
            .iter()
            .map(|v| v.replace("${account}", account))
            .collect();
        for value in s.env.values_mut() {
            *value = value.replace("${account}", account);
        }
        s
    }
}
impl ProviderConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 262144 {
            return Err("provider exceeds 256 KiB".into());
        }
        let raw = json::parse(bytes)?;
        if raw["schemaVersion"].as_u64() != Some(1) {
            return Err("/schemaVersion: only 1 is understood".into());
        }
        let id = text(&raw, "id", 40)?.to_string();
        if !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || id.starts_with('-')
        {
            return Err("/id: expected lowercase letters, digits or hyphens".into());
        }
        let name = text(&raw, "name", 128)?.to_string();
        let revision = bound(&raw, "revision", 1, 1, u64::MAX)?;
        let label = raw["iconLabel"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| id.chars().take(3).collect::<String>().to_uppercase());
        if label.is_empty()
            || label.len() > 3
            || !label
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            return Err("/iconLabel: expected 1 to 3 uppercase letters or digits".into());
        }
        let hex = raw["iconColor"]
            .as_str()
            .filter(|s| s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .unwrap_or("6e6e6e");
        let color = [
            u8::from_str_radix(&hex[0..2], 16).unwrap(),
            u8::from_str_radix(&hex[2..4], 16).unwrap(),
            u8::from_str_radix(&hex[4..6], 16).unwrap(),
        ];
        let source = Source::parse(&raw["source"])?;
        let interval = raw["refresh"]["intervalSeconds"]
            .as_i64()
            .unwrap_or(300)
            .clamp(60, 86400) as u64;
        let mapping = raw["map"].clone();
        let meters = mapping["meters"]
            .as_array()
            .filter(|a| !a.is_empty() && a.len() <= 16)
            .ok_or("/map/meters: expected 1 to 16 meters")?;
        for m in meters {
            let windows = m["windows"]
                .as_array()
                .filter(|a| !a.is_empty() && a.len() <= 16)
                .ok_or("/windows: expected 1 to 16 windows")?;
            for w in windows {
                if let Some(a) = w.get("used") {
                    if ![
                        "percent",
                        "fraction",
                        "remainingFraction",
                        "remainingPercent",
                    ]
                    .contains(&a["as"].as_str().unwrap_or(""))
                    {
                        return Err("/used/as: invalid conversion".into());
                    }
                }
                if let Some(a) = w.get("resetsAt") {
                    if !["iso8601", "epochSeconds", "epochMillis"]
                        .contains(&a["as"].as_str().unwrap_or(""))
                    {
                        return Err("/resetsAt/as: invalid conversion".into());
                    }
                }
            }
        }
        validate_spec(&mapping, 0)?;
        let accounts = raw
            .get("accounts")
            .map(|a| -> Result<Value, String> {
                Source::parse(&a["source"])?;
                if !pointer(&a["each"]) || !pointer(&a["id"]) {
                    return Err("/accounts: expected each/id pointers".into());
                }
                predicates(&a["match"])?;
                Ok(a.clone())
            })
            .transpose()?;
        Ok(Self {
            id,
            name,
            revision,
            label,
            color,
            interval,
            source,
            accounts,
            mapping,
        })
    }
}
