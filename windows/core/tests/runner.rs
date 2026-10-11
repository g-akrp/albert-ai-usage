use ai_usage_core::{
    config::Source,
    runner::{capture, Cancellation},
};
use serde_json::json;

fn source(script: &str) -> Source {
    Source::parse(&json!({"type":"command","executable":"powershell.exe","args":["-NoProfile","-NonInteractive","-Command",script],"timeoutSeconds":3})).unwrap()
}

#[test]
fn hidden_capture_parses_utf8_and_filters_parent_tokens() {
    let s=source("if ($env:GH_TOKEN -or $env:GITHUB_TOKEN) { exit 3 }; [Console]::WriteLine('{\"ok\":true}')");
    assert_eq!(capture(&s, &Cancellation::default()).unwrap()["ok"], true);
}

#[test]
fn bounded_output_fails_and_timeout_cleans_owned_tree() {
    let mut s = source("[Console]::Write(('x' * 10000))");
    s.max_output = 1024;
    assert!(capture(&s, &Cancellation::default())
        .unwrap_err()
        .contains("bytes"));
    let mut s = source("Start-Sleep -Seconds 15");
    s.timeout = 1;
    let start = std::time::Instant::now();
    assert!(capture(&s, &Cancellation::default())
        .unwrap_err()
        .contains("timed out"));
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn stdio_skips_noise_and_correlates_messages() {
    let source=Source::parse(&json!({"type":"stdio","executable":"powershell.exe","args":["-NoProfile","-NonInteractive","-Command","$null=[Console]::ReadLine(); [Console]::WriteLine('noise'); [Console]::WriteLine('{\"id\":0}'); [Console]::WriteLine('{\"id\":2,\"result\":{\"ok\":true}}')"],"timeoutSeconds":5,"steps":[{"write":{"id":2}},{"await":{"match":[{"path":"/id","equals":2}],"require":"/result","capture":"limits"}}],"output":"limits"})).unwrap();
    assert_eq!(
        capture(&source, &Cancellation::default()).unwrap()["result"]["ok"],
        true
    );
}

#[test]
fn stalled_stdin_obeys_whole_run_timeout() {
    let source=Source::parse(&json!({"type":"stdio","executable":"powershell.exe","args":["-NoProfile","-NonInteractive","-Command","Start-Sleep -Seconds 7"],"timeoutSeconds":1,"steps":[{"write":{"payload":"x".repeat(131072)}},{"await":{"capture":"usage"}}],"output":"usage"})).unwrap();
    let start = std::time::Instant::now();
    assert!(capture(&source, &Cancellation::default()).is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(4));
}
