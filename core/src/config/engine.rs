//! Executes a config's `source` (spawns the real provider CLI) and
//! returns the captured JSON document. This is the only place that
//! touches a child process; `mapping.rs` and `pointer.rs` are pure.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::pointer::resolve;
use super::schema::{AwaitSpec, Expect, Predicate, Source, Step};

/// True when every predicate holds against `value`. An empty list
/// (no `match`) holds vacuously.
fn matches(value: &serde_json::Value, predicates: &[Predicate]) -> bool {
    predicates.iter().all(|p| predicate_holds(value, p))
}

fn predicate_holds(value: &serde_json::Value, predicate: &Predicate) -> bool {
    let resolved = resolve(value, &predicate.path);
    if let Some(exists) = predicate.exists {
        return resolved.is_some() == exists;
    }
    if let Some(expected) = &predicate.equals {
        return match (resolved, expected) {
            (Some(serde_json::Value::Number(a)), serde_json::Value::Number(b)) => {
                a.as_f64() == b.as_f64()
            }
            (Some(a), b) => a == b,
            (None, _) => false,
        };
    }
    false
}

/// `error`/`require` checks shared by `command`'s `expect` and
/// `stdio`'s `await`.
fn check_error(value: &serde_json::Value, error_pointer: &Option<String>) -> Result<(), String> {
    let Some(ptr) = error_pointer else {
        return Ok(());
    };
    let Some(err_val) = resolve(value, ptr) else {
        return Ok(());
    };
    if err_val.is_null() {
        return Ok(());
    }
    if let Some(code) = err_val.get("code").and_then(|c| c.as_i64()) {
        if code == -32601 {
            return Err("Update the CLI to see usage".to_string());
        }
    }
    if let Some(msg) = err_val.get("message").and_then(|m| m.as_str()) {
        return Err(msg.to_string());
    }
    if let Some(text) = err_val.as_str() {
        return Err(text.to_string());
    }
    Err("provider reported an error".to_string())
}

fn check_require(
    value: &serde_json::Value,
    require_pointer: &Option<String>,
) -> Result<(), String> {
    let Some(ptr) = require_pointer else {
        return Ok(());
    };
    match resolve(value, ptr) {
        Some(v) if !v.is_null() => Ok(()),
        _ => Err(format!("required field {ptr} missing from response")),
    }
}

/// `~/` at the start of an env value means the home folder, per the
/// schema (`env` values: "a leading ~/ means your home folder").
fn expand_home(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    value.to_string()
}

/// Clears the child's environment and rebuilds only the curated
/// minimal set the spec documents (`example/AGENTS.md` "How a
/// provider runs"), then layers the config's own `env` overrides on
/// top. This is a real correctness property, not just isolation: a
/// stray `GITHUB_TOKEN`/`GH_TOKEN` in the parent shell silently
/// shadows a working `gh` login (confirmed live while investigating
/// the Copilot source) -- omitting it from the minimal set fixes that
/// structurally, for every provider, not as a per-provider workaround.
fn apply_minimal_env(cmd: &mut Command, overrides: &std::collections::HashMap<String, String>) {
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    cmd.env("TERM", "dumb");
    cmd.env("NO_COLOR", "1");
    for key in [
        "USER",
        "LOGNAME",
        "LC_ALL",
        "TMPDIR",
        "SHELL",
        "SSH_AUTH_SOCK",
        "__CF_USER_TEXT_ENCODING",
        // Project extension beyond example/AGENTS.md's documented set:
        // proxy routing config, not a credential. Confirmed live this
        // machine needs it -- a corporate https_proxy is required for
        // any outbound HTTPS call (gh's "no route to host" without
        // it); GITHUB_TOKEN/GH_TOKEN stay excluded, that's the actual
        // stale-credential risk, unrelated to routing.
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "http_proxy",
        "https_proxy",
        "NO_PROXY",
        "no_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(value) = std::env::var(key) {
            cmd.env(key, value);
        }
    }
    cmd.env(
        "LANG",
        std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string()),
    );
    for (k, v) in overrides {
        cmd.env(k, expand_home(v));
    }
}

/// Runs `source` and returns the JSON document the config's `map`
/// should be applied to.
pub fn run_source(source: &Source) -> Result<serde_json::Value, String> {
    match source {
        Source::Command {
            executable,
            args,
            env,
            timeout_seconds,
            expect,
        } => run_command(executable, args, env, *timeout_seconds, expect),
        Source::Stdio {
            executable,
            args,
            env,
            timeout_seconds,
            steps,
            output,
        } => run_stdio(executable, args, env, *timeout_seconds, steps, output),
    }
}

fn run_command(
    executable: &str,
    args: &[String],
    env: &std::collections::HashMap<String, String>,
    timeout_seconds: u32,
    expect: &Option<Expect>,
) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new(executable);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    apply_minimal_env(&mut cmd, env);
    let mut child = cmd
        .spawn()
        .map_err(|_| format!("could not start {executable}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "no stdout handle".to_string())?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buf = String::new();
        let _ = stdout.read_to_string(&mut buf);
        let _ = tx.send(buf);
    });

    let stdout_text = match rx.recv_timeout(Duration::from_secs(timeout_seconds as u64)) {
        Ok(text) => text,
        Err(_) => {
            let _ = child.kill();
            return Err(format!("{executable} timed out"));
        }
    };
    let status = child
        .wait()
        .map_err(|_| "could not wait for provider process".to_string())?;
    if !status.success() {
        return Err(format!("{executable} exited with a non-zero status"));
    }

    let value: serde_json::Value = serde_json::from_str(&stdout_text)
        .map_err(|_| format!("{executable} output was not valid JSON"))?;

    if let Some(expect) = expect {
        if !matches(&value, &expect.match_) {
            return Err("output did not match the expected shape".to_string());
        }
        check_error(&value, &expect.error)?;
        check_require(&value, &expect.require)?;
    }
    Ok(value)
}

fn run_stdio(
    executable: &str,
    args: &[String],
    env: &std::collections::HashMap<String, String>,
    timeout_seconds: u32,
    steps: &[Step],
    output: &str,
) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new(executable);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    apply_minimal_env(&mut cmd, env);
    let mut child = cmd
        .spawn()
        .map_err(|_| format!("could not start {executable}"))?;

    let mut stdin = Some(
        child
            .stdin
            .take()
            .ok_or_else(|| "no stdin handle".to_string())?,
    );
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "no stdout handle".to_string())?;
    let mut reader = BufReader::new(stdout);

    let (tx, rx) = mpsc::channel::<Option<String>>();
    thread::spawn(move || loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let _ = tx.send(None);
                break;
            }
            Ok(_) => {
                if tx.send(Some(line)).is_err() {
                    break;
                }
            }
            Err(_) => {
                let _ = tx.send(None);
                break;
            }
        }
    });

    let timeout = Duration::from_secs(timeout_seconds as u64);
    let mut captures: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();

    for step in steps {
        match step {
            Step::Write(w) => {
                let line = w.write.to_string();
                if let Some(stdin) = stdin.as_mut() {
                    stdin
                        .write_all(line.as_bytes())
                        .and_then(|_| stdin.write_all(b"\n"))
                        .map_err(|_| format!("could not write to {executable}"))?;
                }
            }
            Step::Await(a) => {
                let answer = await_line(&rx, timeout, &a.await_)?;
                check_error(&answer, &a.await_.error)?;
                check_require(&answer, &a.await_.require)?;
                if let Some(name) = &a.await_.capture {
                    captures.insert(name.clone(), answer);
                }
            }
        }
    }

    #[allow(unused_assignments)]
    {
        stdin = None; // close stdin once all steps are done, signal EOF
    }
    let _ = child.kill();
    let _ = child.wait();

    captures
        .remove(output)
        .ok_or_else(|| format!("no captured value named '{output}'"))
}

/// Reads lines (skipping ones that don't match, e.g. hook/notification
/// events interleaved with the real answer -- observed live for both
/// Codex and Claude Code) until one matches `spec.match_`, or gives up
/// after `timeout` / an unreasonable number of lines.
fn await_line(
    rx: &mpsc::Receiver<Option<String>>,
    timeout: Duration,
    spec: &AwaitSpec,
) -> Result<serde_json::Value, String> {
    const MAX_LINES: u32 = 500;
    for _ in 0..MAX_LINES {
        let line = match rx.recv_timeout(timeout) {
            Ok(Some(line)) => line,
            Ok(None) => return Err("provider closed its output stream".to_string()),
            Err(_) => return Err("timed out waiting for a response".to_string()),
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue; // "Lines that don't match an await are skipped,
                      // including lines that aren't JSON."
        };
        if matches(&value, &spec.match_) {
            return Ok(value);
        }
    }
    Err("too many unmatched lines before a response".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn predicate_equals_matches_number_regardless_of_int_or_float_repr() {
        let value = json!({"id": 2});
        let p = Predicate {
            path: "/id".to_string(),
            equals: Some(json!(2.0)),
            exists: None,
        };
        assert!(predicate_holds(&value, &p));
    }

    #[test]
    fn predicate_equals_string_is_exact() {
        let value = json!({"id": "2"});
        let p = Predicate {
            path: "/id".to_string(),
            equals: Some(json!(2)),
            exists: None,
        };
        assert!(!predicate_holds(&value, &p));
    }

    #[test]
    fn predicate_exists_true() {
        let value = json!({"result": {}});
        let p = Predicate {
            path: "/result".to_string(),
            equals: None,
            exists: Some(true),
        };
        assert!(predicate_holds(&value, &p));
    }

    #[test]
    fn check_error_null_is_ok() {
        let value = json!({"error": null});
        assert!(check_error(&value, &Some("/error".to_string())).is_ok());
    }

    #[test]
    fn check_error_code_dash_32601_has_friendly_message() {
        let value = json!({"error": {"code": -32601}});
        let err = check_error(&value, &Some("/error".to_string())).unwrap_err();
        assert_eq!(err, "Update the CLI to see usage");
    }

    #[test]
    fn check_require_missing_fails() {
        let value = json!({});
        assert!(check_require(&value, &Some("/result".to_string())).is_err());
    }

    #[test]
    fn expand_home_substitutes_home_prefix() {
        std::env::set_var("HOME", "/Users/testuser");
        assert_eq!(expand_home("~/bin/tool"), "/Users/testuser/bin/tool");
        assert_eq!(expand_home("/usr/bin/tool"), "/usr/bin/tool");
    }
}
