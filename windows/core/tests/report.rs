use ai_usage_core::{config::ProviderConfig, report::Report};
use serde_json::json;

#[test]
fn copilot_counts_units_filters_and_root_reset() {
    let config =
        ProviderConfig::parse(include_bytes!("../../../Resources/providers/copilot.json")).unwrap();
    let data = json!({"copilot_plan":"pro","quota_reset_date_utc":"2026-11-01T00:00:00Z","quota_snapshots":{
        "chat":{"has_quota":true,"unlimited":true,"percent_remaining":100},
        "completions":{"has_quota":false,"unlimited":false,"percent_remaining":99},
        "premium_interactions":{"has_quota":false,"unlimited":false,"entitlement":300,"quota_remaining":246,"percent_remaining":82,"token_based_billing":true}
    }});
    let report = Report::map(&config.mapping, &data).unwrap();
    assert_eq!(report.meters.len(), 1);
    assert_eq!(report.headline(), Some(18.0));
    let window = &report.meters[0].windows[0];
    assert_eq!(window.resets_at, Some(1793491200));
    let count = window.count.as_ref().unwrap();
    assert_eq!((count.used, count.limit), (54.0, 300.0));
    assert_eq!(count.unit.as_deref(), Some("credits"));
}

#[test]
fn codex_maps_session_priority_and_weekly_warning_independently() {
    let config =
        ProviderConfig::parse(include_bytes!("../../../Resources/providers/codex.json")).unwrap();
    let data = json!({"result":{"rateLimits":{"limitId":"codex","limitName":"Codex","planType":"plus","primary":{"usedPercent":26,"resetsAt":1000,"windowDurationMins":300},"secondary":{"usedPercent":82,"windowDurationMins":10080}},"ordinaryUsageAllowed":true}});
    let report = Report::map(&config.mapping, &data).unwrap();
    assert_eq!(report.headline(), Some(26.0));
    assert_eq!(report.max_percent(), Some(82.0));
    assert_eq!(report.plan.as_deref(), Some("plus"));
}

#[test]
fn dynamic_group_scopes_and_clamping() {
    let config = ProviderConfig::parse(include_bytes!(
        "../../../Resources/providers/antigravity.json"
    ))
    .unwrap();
    let report=Report::map(&config.mapping,&json!({"command":{"data":{"groups":[{"name":"Gemini Models","buckets":[{"id":"s","name":"Five Hour","remaining_fraction":0.37,"window":"FIVE_HOUR"},{"id":"w","name":"Weekly","remaining_fraction":-0.2,"window":"WEEKLY"}]}]}}})).unwrap();
    assert_eq!(report.meters[0].windows.len(), 2);
    assert_eq!(report.max_percent(), Some(100.0));
    assert!(Report::map(&config.mapping, &json!({})).is_err());
}
