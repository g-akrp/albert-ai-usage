use serde_json::Value;

pub fn parse(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
        .map_err(|_| "not valid JSON".into())
}
pub fn resolve<'a>(value: &'a Value, pointer: &str) -> Option<&'a Value> {
    value.pointer(pointer)
}
pub fn number(value: Option<&Value>) -> Option<f64> {
    let n = match value? {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => s.parse().ok()?,
        _ => return None,
    };
    n.is_finite().then_some(n)
}
pub fn string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
pub fn matches(spec: &Value, value: &Value) -> bool {
    spec.as_array().is_none_or(|predicates| {
        predicates.iter().all(|p| {
            let found = p["path"].as_str().and_then(|s| value.pointer(s));
            if let Some(wanted) = p.get("equals") {
                found.is_some_and(|v| v == wanted)
            } else if let Some(wanted) = p.get("notEquals") {
                found.is_none_or(|v| v != wanted)
            } else {
                p["exists"]
                    .as_bool()
                    .is_some_and(|wanted| wanted == found.is_some_and(|v| !v.is_null()))
            }
        })
    })
}
