use ai_usage_core::relative_time::{parse_date, time_left};

#[test]
fn dates_and_compact_countdowns_handle_offsets_and_boundaries() {
    assert_eq!(parse_date("2026-11-01T00:00:00Z"), Some(1793491200));
    assert_eq!(parse_date("2026-11-01T07:00:00+07:00"), Some(1793491200));
    assert_eq!(time_left(205 * 60, 0), "in 3h 25m");
    assert_eq!(time_left(4 * 86400 + 2 * 3600 + 12 * 60, 0), "in 4d 2h 12m");
    assert_eq!(time_left(0, 0), "resetting…");
    assert_eq!(time_left(15, 0), "in 15s");
    assert_eq!(time_left(125, 0), "in 2m");
    assert_eq!(parse_date("2026-02-30T00:00:00Z"), None);
    assert_eq!(parse_date("2026-11-01T-1:00:00Z"), None);
    assert_eq!(parse_date("2026-11-01T00:00:00+-1:00"), None);
}
