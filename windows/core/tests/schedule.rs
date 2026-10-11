use ai_usage_core::schedule::Schedule;

#[test]
fn backoff_duplicate_suppression_and_two_worker_limit() {
    let mut s = Schedule::default();
    assert!(s.reserve("claude", 0));
    assert!(!s.reserve("claude", 0));
    assert!(s.reserve("codex", 0));
    assert!(!s.reserve("agy", 0));
    s.finish("claude", 0, 300, false);
    assert!(!s.reserve("claude", 59));
    assert!(s.reserve("claude", 60));
    s.finish("claude", 60, 300, false);
    assert!(!s.reserve("claude", 179));
    assert!(s.reserve("claude", 180));
    s.finish("claude", 180, 300, true);
    assert!(!s.reserve("claude", 479));
    assert!(s.reserve("claude", 480));
}

#[test]
fn manual_refresh_while_running_queues_one_followup() {
    let mut s = Schedule::default();
    assert!(s.reserve("claude", 0));
    s.refresh();
    s.refresh();
    s.finish("claude", 1, 300, true);
    assert!(s.reserve("claude", 1));
    assert!(!s.reserve("claude", 1));
}

#[test]
fn running_and_retry_in_follow_failures() {
    let mut s = Schedule::default();
    assert!(!s.running("claude"));
    assert!(!s.any_running());
    assert!(s.reserve("claude", 0));
    assert!(s.running("claude"));
    assert!(s.any_running());
    assert_eq!(s.retry_in("claude", 0), None);
    s.finish("claude", 10, 300, false);
    assert!(!s.running("claude"));
    assert_eq!(s.retry_in("claude", 10), Some(60));
    assert_eq!(s.retry_in("claude", 100), Some(0));
    s.retry("claude");
    assert!(s.reserve("claude", 11));
    s.finish("claude", 12, 300, true);
    assert_eq!(s.retry_in("claude", 12), None);
}

#[test]
fn retry_while_running_queues_one_followup() {
    let mut s = Schedule::default();
    assert!(s.reserve("codex", 0));
    s.retry("codex");
    s.finish("codex", 1, 300, false);
    assert!(s.reserve("codex", 1));
}
