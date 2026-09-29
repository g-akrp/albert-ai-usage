//! Shared provider contract. See docs/architecture.md and
//! docs/core-abi.md (C-ABI deferred until a shell exists; V0.1.0 proves
//! the port by calling it in-process from the CLI).

use serde::Serialize;

/// Normalized provider status. Collapses provider-specific error causes
/// (auth, timeout, parse, rate-limit, unknown) into one sanitized
/// `Error(String)` variant — never leak raw provider error text or
/// credential/token content through this field.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", content = "detail")]
pub enum ProviderStatus {
    /// Live, verified data is present in this snapshot.
    Available,
    /// Provider has an unverified or not-yet-approved source; fixture or
    /// sample data may be attached for illustration, never live data.
    Pending,
    /// No approved source exists for this provider at all.
    Unsupported,
    /// A sanitized, human-readable failure reason. Never a raw provider
    /// error string, stack trace, or anything that could carry a token.
    Error(String),
}

/// A used/limit credit count (e.g. GitHub Copilot premium interactions:
/// "42/300"), for providers that report quota as a count rather than a
/// percentage. `label` names the count (e.g. "premium_interactions").
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageCount {
    pub label: String,
    pub used: u32,
    pub limit: u32,
}

/// Normalized usage snapshot every provider returns, regardless of its
/// underlying source shape. Only sanitized, allowlisted fields belong
/// here — never account identifiers, credentials, or raw responses.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageSnapshot {
    pub provider: String,
    pub status: ProviderStatus,
    pub session_usage_percent: Option<f32>,
    pub session_reset_label: Option<String>,
    pub weekly_usage_percent: Option<f32>,
    pub weekly_reset_label: Option<String>,
    /// Used/limit count quotas (e.g. Copilot premium interactions).
    /// Separate from the percent fields since not every provider's
    /// quota is expressible as a percentage.
    pub counts: Vec<UsageCount>,
    /// Short, sanitized note for the human (e.g. "fixture data, not
    /// live" or "mock provider"). Never raw provider output.
    pub note: Option<String>,
}

/// Sanitized error a provider can return instead of a snapshot. Same
/// sanitization rule as `ProviderStatus::Error`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderError(pub String);

/// The port every provider adapter implements. Synchronous for V0.1.0:
/// the CLI does one-shot sequential/isolated calls, no concurrent
/// refresh requirement yet.
///
/// `id` returns an owned `String`, not `&'static str`: a provider type
/// can have more than one live instance (e.g. one Copilot adapter per
/// discovered GitHub account), each needing its own id.
pub trait UsageProvider {
    fn id(&self) -> String;
    fn display_name(&self) -> String;
    fn fetch_usage(&self) -> Result<UsageSnapshot, ProviderError>;
}
