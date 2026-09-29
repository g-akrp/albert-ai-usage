//! Live call to the local `codex app-server`. Off by default — only
//! runs when `ALBERT_LIVE_CODEX=1` is set (`make run-live`). No
//! auth-state file is read, no credential is inspected; only the
//! JSON-RPC stdio channel is used. Any failure degrades to a sanitized
//! reason string — this never panics, never blocks past `READ_TIMEOUT`.
//!
//! WIRE FORMAT NOTE: newline-delimited JSON-RPC 2.0 is this
//! implementation's best-effort assumption from public research
//! (docs/provider-source-proposals.md), not a confirmed spec against
//! the real binary. `extract_result_field` normalizes both plausible
//! response shapes it could return.

use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Lists every GitHub account `gh` currently has stored, by reading
/// `~/.config/gh/hosts.yml` directly (no documented `gh` command
/// prints a clean list — `gh auth status` is human text only).
///
/// KNOWN LIMITATION: this is a best-effort parse of gh's own config
/// file format, not a stable public API; it could break on a future
/// `gh` version. Also macOS/Linux only — Windows stores this at
/// `%APPDATA%\GitHub CLI\hosts.yml`, not handled here (V0.1.0 has no
/// Windows shell yet).
pub fn list_gh_accounts() -> Result<Vec<String>, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let path = format!("{home}/.config/gh/hosts.yml");
    let contents =
        std::fs::read_to_string(&path).map_err(|_| "could not read gh hosts config".to_string())?;

    let accounts = parse_gh_accounts(&contents);
    if accounts.is_empty() {
        return Err("no gh accounts found".to_string());
    }
    Ok(accounts)
}

/// Pure parser for `gh`'s `hosts.yml` shape:
/// ```yaml
/// github.com:
///     git_protocol: https
///     users:
///         g-akrp:
///         2521180709_bblghcp:
///     user: 2521180709_bblghcp
/// ```
/// Account names are indented under `users:` and end with `:` with no
/// value on the same line. Separated from `list_gh_accounts` so the
/// parsing logic is unit-testable without touching the real file.
fn parse_gh_accounts(yaml: &str) -> Vec<String> {
    let mut accounts = Vec::new();
    let mut in_users_block = false;
    for line in yaml.lines() {
        if line.trim_start() == "users:" {
            in_users_block = true;
            continue;
        }
        if in_users_block {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            if indent >= 8 && trimmed.ends_with(':') && !trimmed.contains(' ') {
                accounts.push(trimmed.trim_end_matches(':').to_string());
            } else if !trimmed.is_empty() {
                in_users_block = false;
            }
        }
    }
    accounts
}

/// Live call to `gh api copilot_internal/user` for one specific
/// account. Off by default — only runs when `ALBERT_LIVE_COPILOT=1`
/// is set (`make run-live-copilot`). No credential file is read here:
/// `gh auth token --user <account>` reads that account's already-
/// stored token without switching gh's global active account or
/// mutating any state.
///
/// A stale `GITHUB_TOKEN`/`GH_TOKEN` env var can shadow the intended
/// account and cause a generic 401 — see docs/data-source/copilot.md.
/// Both are removed for this child process only; the parent shell's
/// environment is never touched. Non-zero exit degrades to a
/// sanitized error, never a raw stderr dump.
pub fn call_copilot_user_for_account(account: &str) -> Result<String, String> {
    let token_output = Command::new("gh")
        .args(["auth", "token", "--user", account])
        .output()
        .map_err(|_| "could not run gh auth token".to_string())?;
    if !token_output.status.success() {
        return Err(format!("no stored gh token for account {account}"));
    }
    let token = String::from_utf8(token_output.stdout)
        .map_err(|_| "gh auth token output was not valid UTF-8".to_string())?;

    let output = Command::new("gh")
        .args(["api", "copilot_internal/user"])
        .env_remove("GITHUB_TOKEN")
        .env("GH_TOKEN", token.trim())
        .output()
        .map_err(|_| "could not run gh".to_string())?;

    if !output.status.success() {
        return Err("gh authentication or API call failed".to_string());
    }

    String::from_utf8(output.stdout).map_err(|_| "gh output was not valid UTF-8".to_string())
}

const HANDSHAKE_DELAY: Duration = Duration::from_millis(500);
const READ_TIMEOUT: Duration = Duration::from_secs(5);

pub fn call_codex_rate_limits() -> Result<String, String> {
    let mut child = Command::new("codex")
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "could not start codex app-server".to_string())?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "no stdin handle".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "no stdout handle".to_string())?;
    let mut reader = BufReader::new(stdout);

    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    thread::spawn(move || loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let _ = tx.send(Err("app-server closed stdout".to_string()));
                break;
            }
            Ok(_) => {
                if tx.send(Ok(line)).is_err() {
                    break;
                }
            }
            Err(_) => {
                let _ = tx.send(Err("read error from app-server".to_string()));
                break;
            }
        }
    });

    // Envelope and method names confirmed against the authoritative
    // schema (`codex app-server generate-json-schema --experimental`):
    // no top-level "jsonrpc" field, and `initialize` requires
    // `params.clientInfo.{name,version}`.
    write_line(
        &mut stdin,
        r#"{"id":1,"method":"initialize","params":{"clientInfo":{"name":"albert-ai-usage","version":"0.1.0"}}}"#,
    )?;
    recv_response_for_id(&rx, 1)?; // handshake ack, content not needed

    thread::sleep(HANDSHAKE_DELAY);

    write_line(
        &mut stdin,
        r#"{"id":2,"method":"account/rateLimits/read","params":null}"#,
    )?;
    let response = recv_response_for_id(&rx, 2)?;

    let _ = child.kill();
    let _ = child.wait();

    extract_result_field(&response)
}

fn write_line(stdin: &mut ChildStdin, line: &str) -> Result<(), String> {
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .map_err(|_| "could not write to app-server".to_string())
}

/// The server interleaves unsolicited notifications (no `"id"` field,
/// e.g. `remoteControl/status/changed`) between responses. Reads lines
/// until one is a response whose `"id"` matches, skipping the rest.
/// Bounded both by `READ_TIMEOUT` per read and `MAX_LINES` total, so a
/// pathological notification stream can't hang this forever.
const MAX_LINES: u32 = 20;

fn recv_response_for_id(
    rx: &mpsc::Receiver<Result<String, String>>,
    id: u64,
) -> Result<String, String> {
    for _ in 0..MAX_LINES {
        let line = match rx.recv_timeout(READ_TIMEOUT) {
            Ok(Ok(line)) => line,
            Ok(Err(reason)) => return Err(reason),
            Err(_) => return Err("timed out waiting for app-server response".to_string()),
        };
        if is_response_for(&line, id) {
            return Ok(line);
        }
        // else: an unsolicited notification, keep reading.
    }
    Err("too many notifications before a matching response".to_string())
}

/// True when `line` is a JSON-RPC response (not a notification) whose
/// `"id"` matches `id`.
fn is_response_for(line: &str, id: u64) -> bool {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|v| v.get("id").and_then(|i| i.as_u64()))
        == Some(id)
}

/// Pulls the `"result"` object out of a JSON-RPC 2.0 response envelope
/// and normalizes it to the `{"rateLimits": {...}}` shape `codex::
/// parse_rate_limits` expects, whether or not the real server already
/// wraps it that way.
fn extract_result_field(line: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|_| "malformed JSON-RPC response".to_string())?;
    let result = value
        .get("result")
        .cloned()
        .ok_or_else(|| "response had no result field".to_string())?;
    let envelope = if result.get("rateLimits").is_some() {
        result
    } else {
        serde_json::json!({ "rateLimits": result })
    };
    Ok(envelope.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_accounts_from_real_hosts_yml_shape() {
        let yaml = "github.com:\n    git_protocol: https\n    users:\n        g-akrp:\n        2521180709_bblghcp:\n    user: 2521180709_bblghcp\n";
        let accounts = parse_gh_accounts(yaml);
        assert_eq!(accounts, vec!["g-akrp", "2521180709_bblghcp"]);
    }

    #[test]
    fn parses_single_account() {
        let yaml = "github.com:\n    users:\n        only-one:\n    user: only-one\n";
        assert_eq!(parse_gh_accounts(yaml), vec!["only-one"]);
    }

    #[test]
    fn no_users_block_returns_empty() {
        assert_eq!(
            parse_gh_accounts("github.com:\n    git_protocol: https\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn passes_through_when_already_wrapped() {
        let line = r#"{"jsonrpc":"2.0","id":2,"result":{"rateLimits":{"primary":{}}}}"#;
        let out = extract_result_field(line).unwrap();
        assert!(out.contains("rateLimits"));
        assert!(out.contains("primary"));
    }

    #[test]
    fn wraps_bare_result_in_rate_limits_key() {
        let line = r#"{"jsonrpc":"2.0","id":2,"result":{"primary":{},"secondary":{}}}"#;
        let out = extract_result_field(line).unwrap();
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(value.get("rateLimits").unwrap().get("primary").is_some());
    }

    #[test]
    fn missing_result_field_is_a_sanitized_error() {
        let line = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-1}}"#;
        let err = extract_result_field(line).unwrap_err();
        assert_eq!(err, "response had no result field");
    }

    #[test]
    fn malformed_json_is_a_sanitized_error() {
        let err = extract_result_field("not json").unwrap_err();
        assert_eq!(err, "malformed JSON-RPC response");
    }

    #[test]
    fn is_response_for_matches_id() {
        assert!(is_response_for(r#"{"id":2,"result":{}}"#, 2));
        assert!(!is_response_for(r#"{"id":1,"result":{}}"#, 2));
    }

    #[test]
    fn is_response_for_rejects_notifications_without_id() {
        assert!(!is_response_for(
            r#"{"method":"account/updated","params":{}}"#,
            2
        ));
    }

    #[test]
    fn is_response_for_rejects_malformed_lines() {
        assert!(!is_response_for("not json", 2));
    }
}
