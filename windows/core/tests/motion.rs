use ai_usage_core::motion::{ease_out_cubic, Motion, OPEN_MS, RING_MS, UPDATED_MS};

#[test]
fn easing_starts_at_zero_ends_at_one_and_decelerates() {
    assert_eq!(ease_out_cubic(0.0), 0.0);
    assert_eq!(ease_out_cubic(1.0), 1.0);
    assert_eq!(ease_out_cubic(-1.0), 0.0);
    assert_eq!(ease_out_cubic(2.0), 1.0);
    assert!(ease_out_cubic(0.5) > 0.5);
}

#[test]
fn open_progress_runs_for_open_duration() {
    let mut m = Motion::new(false);
    assert_eq!(m.open_progress(0.0), 1.0);
    m.open(1000.0);
    assert_eq!(m.open_progress(1000.0), 0.0);
    assert!(m.open_progress(1000.0 + OPEN_MS / 2.0) > 0.5);
    assert_eq!(m.open_progress(1000.0 + OPEN_MS), 1.0);
    assert!(m.active(1000.0 + OPEN_MS / 2.0));
    assert!(!m.active(1000.0 + OPEN_MS));
}

#[test]
fn first_ring_sweeps_from_zero_and_unchanged_ring_does_not_move() {
    let mut m = Motion::new(false);
    m.rings(&[("claude#0#0".into(), Some(40.0))], 0.0);
    assert_eq!(m.ring_value("claude#0#0", 0.0, Some(40.0)), Some(0.0));
    let mid = m
        .ring_value("claude#0#0", RING_MS / 2.0, Some(40.0))
        .unwrap();
    assert!(mid > 20.0 && mid < 40.0);
    assert_eq!(m.ring_value("claude#0#0", RING_MS, Some(40.0)), Some(40.0));
    m.rings(&[("claude#0#0".into(), Some(40.0))], 5000.0);
    assert_eq!(m.ring_value("claude#0#0", 5000.0, Some(40.0)), Some(40.0));
    assert!(!m.active(5000.0));
}

#[test]
fn changed_ring_sweeps_from_last_shown_value() {
    let mut m = Motion::new(false);
    m.rings(&[("codex#0#0".into(), Some(40.0))], 0.0);
    m.rings(&[("codex#0#0".into(), Some(70.0))], 10_000.0);
    assert_eq!(m.ring_value("codex#0#0", 10_000.0, Some(70.0)), Some(40.0));
    assert_eq!(
        m.ring_value("codex#0#0", 10_000.0 + RING_MS, Some(70.0)),
        Some(70.0)
    );
}

#[test]
fn ring_change_mid_sweep_continues_from_current_value() {
    let mut m = Motion::new(false);
    m.rings(&[("a".into(), Some(100.0))], 0.0);
    let now = RING_MS / 2.0;
    let current = m.ring_value("a", now, Some(100.0)).unwrap();
    m.rings(&[("a".into(), Some(10.0))], now);
    assert_eq!(m.ring_value("a", now, Some(10.0)), Some(current));
}

#[test]
fn ring_without_percent_is_not_animated() {
    let mut m = Motion::new(false);
    m.rings(&[("a".into(), None)], 0.0);
    assert_eq!(m.ring_value("a", 0.0, None), None);
    assert!(!m.active(0.0));
}

#[test]
fn updated_glow_fades_from_one_to_zero() {
    let mut m = Motion::new(false);
    assert_eq!(m.updated_glow(0.0), 0.0);
    m.flash_updated(0.0);
    assert_eq!(m.updated_glow(0.0), 1.0);
    assert!(m.updated_glow(UPDATED_MS / 2.0) > 0.0);
    assert_eq!(m.updated_glow(UPDATED_MS), 0.0);
}

#[test]
fn reduced_motion_has_no_animation() {
    let mut m = Motion::new(true);
    m.open(0.0);
    m.rings(&[("a".into(), Some(55.0))], 0.0);
    m.flash_updated(0.0);
    assert_eq!(m.open_progress(0.0), 1.0);
    assert_eq!(m.ring_value("a", 0.0, Some(55.0)), Some(55.0));
    assert_eq!(m.updated_glow(0.0), 0.0);
    assert!(!m.active(0.0));
}
