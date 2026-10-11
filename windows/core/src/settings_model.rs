use serde_json::{json, Value};
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub pin: Option<String>,
    pub hidden: Vec<String>,
    pub disabled: Vec<String>,
    pub collapsed: Vec<String>,
    pub order: Vec<String>,
    pub overflow_hint_dismissed: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            pin: None,
            hidden: Vec::new(),
            disabled: vec!["antigravity".into(), "copilot".into()],
            collapsed: Vec::new(),
            order: Vec::new(),
            overflow_hint_dismissed: false,
        }
    }
}
fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .take(128)
                .filter_map(|v| v.as_str().filter(|s| s.len() <= 256).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}
impl Settings {
    pub fn parse(bytes: &[u8]) -> Self {
        let v = crate::json::parse(bytes).unwrap_or(Value::Null);
        Self {
            pin: v["pinnedProvider"]
                .as_str()
                .filter(|s| s.len() <= 256)
                .map(str::to_string),
            hidden: strings(&v["hiddenCards"]),
            disabled: if v["disabledProviders"].is_array() {
                strings(&v["disabledProviders"])
            } else {
                Self::default().disabled
            },
            collapsed: strings(&v["collapsedCards"]),
            order: strings(&v["cardOrder"]),
            overflow_hint_dismissed: v["overflowHintDismissed"].as_bool().unwrap_or(false),
        }
    }
    pub fn to_json(&self) -> String {
        json!({"pinnedProvider":self.pin,"hiddenCards":self.hidden,"disabledProviders":self.disabled,"collapsedCards":self.collapsed,"cardOrder":self.order,"overflowHintDismissed":self.overflow_hint_dismissed}).to_string()
    }
    pub fn toggle(list: &mut Vec<String>, id: &str) {
        if list.iter().any(|s| s == id) {
            list.retain(|s| s != id);
        } else {
            list.push(id.into());
        }
    }
}
