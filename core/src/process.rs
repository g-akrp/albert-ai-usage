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

    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    )?;
    recv_line(&rx)?; // handshake ack, content not needed

    thread::sleep(HANDSHAKE_DELAY);

    write_line(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"account/rateLimits/read","params":{}}"#,
    )?;
    let response = recv_line(&rx)?;

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

fn recv_line(rx: &mpsc::Receiver<Result<String, String>>) -> Result<String, String> {
    match rx.recv_timeout(READ_TIMEOUT) {
        Ok(Ok(line)) => Ok(line),
        Ok(Err(reason)) => Err(reason),
        Err(_) => Err("timed out waiting for app-server response".to_string()),
    }
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
}
