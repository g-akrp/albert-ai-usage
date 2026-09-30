//! Provider config schema -- matches the format documented in
//! `example/AGENTS.md` (Maestri's own shipped Agent Usage provider
//! files). A provider is data, not Rust: add a new one by writing a
//! JSON file, no code change.
//!
//! This is a pragmatic subset: it covers everything needed to run the
//! real shipped examples (claude.json, codex.json, antigravity.json)
//! correctly. GUI-only concerns (ring assignment, colors, popover,
//! `status.json` writing, `maxOutputBytes`/`maxLineBytes` enforcement)
//! are out of scope for this CLI -- V0.1.0 has no GUI.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub id: String,
    pub revision: u32,
    pub name: String,
    /// Project extension beyond the base schema (not in Maestri's
    /// shipped examples): a short label (ideally 3 chars) for the
    /// square menu-bar icon's top row -- `name` is the full display
    /// name used in the dropdown, which is usually too long to fit
    /// there. Falls back to the first 3 characters of `id`, uppercased,
    /// when not set.
    #[serde(default)]
    #[serde(rename = "iconLabel")]
    pub icon_label: Option<String>,
    /// Project extension: the provider's own brand color, hex
    /// `"RRGGBB"` (no `#`), for the icon -- identity at a glance, not
    /// usage severity (the dropdown's per-window percent color still
    /// does that). Falls back to a neutral gray when not set or not
    /// valid hex.
    #[serde(default)]
    #[serde(rename = "iconColor")]
    pub icon_color: Option<String>,
    pub source: Source,
    pub map: Map,
    #[serde(default)]
    pub refresh: Option<Refresh>,
}

#[derive(Debug, Deserialize)]
pub struct Refresh {
    #[serde(rename = "intervalSeconds")]
    pub interval_seconds: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Source {
    #[serde(rename = "command")]
    Command {
        executable: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(rename = "timeoutSeconds", default = "default_command_timeout")]
        timeout_seconds: u32,
        #[serde(default)]
        expect: Option<Expect>,
    },
    #[serde(rename = "stdio")]
    Stdio {
        executable: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(rename = "timeoutSeconds", default = "default_stdio_timeout")]
        timeout_seconds: u32,
        steps: Vec<Step>,
        output: String,
    },
}

fn default_command_timeout() -> u32 {
    30
}
fn default_stdio_timeout() -> u32 {
    15
}

#[derive(Debug, Deserialize)]
pub struct Expect {
    #[serde(default)]
    #[serde(rename = "match")]
    pub match_: Vec<Predicate>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub require: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Step {
    Write(WriteStep),
    Await(AwaitStep),
}

#[derive(Debug, Deserialize)]
pub struct WriteStep {
    pub write: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AwaitStep {
    #[serde(rename = "await")]
    pub await_: AwaitSpec,
}

#[derive(Debug, Deserialize)]
pub struct AwaitSpec {
    #[serde(default)]
    #[serde(rename = "match")]
    pub match_: Vec<Predicate>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub require: Option<String>,
    #[serde(default)]
    pub capture: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Predicate {
    pub path: String,
    #[serde(default)]
    pub equals: Option<serde_json::Value>,
    #[serde(default)]
    pub exists: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Map {
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub available: Option<PathRef>,
    #[serde(default)]
    pub plan: Option<PlanRef>,
    #[serde(default)]
    pub access: Option<PathRef>,
    pub meters: Vec<MeterSpec>,
}

#[derive(Debug, Deserialize)]
pub struct PathRef {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanRef {
    pub path: String,
    #[serde(default)]
    pub names: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct MeterSpec {
    #[serde(flatten)]
    pub scope: Scope,
    pub id: IdSpec,
    pub label: LabelSpec,
    #[serde(default)]
    pub plan: Option<PathRef>,
    #[serde(default)]
    #[serde(rename = "limitReachedReason")]
    pub limit_reached_reason: Option<PathRef>,
    #[serde(default)]
    pub windows: Vec<WindowSpec>,
}

#[derive(Debug, Deserialize)]
pub struct WindowSpec {
    #[serde(flatten)]
    pub scope: Scope,
    pub id: IdSpec,
    #[serde(default)]
    pub label: Option<LabelSpec>,
    #[serde(default)]
    pub used: Option<UsedSpec>,
    #[serde(default)]
    #[serde(rename = "resetsAt")]
    pub resets_at: Option<AsSpec>,
    #[serde(default)]
    pub duration: Option<DurationSpec>,
}

/// At most one of `select` / `each` / `eachEntry`. Missing means
/// "read the current value" (used for a meter/window with no
/// iteration -- e.g. Codex's single `/rateLimits` bucket).
#[derive(Debug, Deserialize, Default)]
pub struct Scope {
    #[serde(default)]
    pub select: Option<String>,
    #[serde(default)]
    pub each: Option<String>,
    #[serde(default)]
    #[serde(rename = "eachEntry")]
    pub each_entry: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum IdSpec {
    Literal(String),
    Path {
        path: String,
    },
    EntryKey {
        #[serde(rename = "entryKey")]
        entry_key: bool,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LabelSpec {
    Text {
        text: String,
    },
    Key {
        key: String,
    },
    PathFallback {
        path: String,
        #[serde(default)]
        fallback: Option<Box<LabelSpec>>,
    },
}

#[derive(Debug, Deserialize)]
pub struct UsedSpec {
    pub path: String,
    #[serde(rename = "as")]
    pub as_: UsedAs,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UsedAs {
    Percent,
    Fraction,
    RemainingFraction,
    /// Project extension beyond the base schema (`example/AGENTS.md`
    /// documents only the three above): a 0..=100 remaining
    /// percentage, for sources that report "percent remaining"
    /// directly rather than a 0..1 fraction (e.g. GitHub Copilot's
    /// `percent_remaining` field -- there is no compiled-in Maestri
    /// example for Copilot to follow instead).
    RemainingPercent,
}

#[derive(Debug, Deserialize)]
pub struct AsSpec {
    pub path: String,
    #[serde(rename = "as")]
    pub as_: ResetAs,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResetAs {
    Iso8601,
    EpochSeconds,
    EpochMillis,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DurationSpec {
    Seconds {
        seconds: u64,
    },
    FromPath {
        path: String,
        #[serde(rename = "as")]
        as_: DurationAs,
    },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DurationAs {
    Seconds,
    Minutes,
    WindowName,
}
