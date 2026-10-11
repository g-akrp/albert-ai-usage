use ai_usage_core::config::ProviderConfig;
use ai_usage_core::config::Source;

#[test]
fn shipped_provider_definitions_parse() {
    for text in [
        include_str!("../../../Resources/providers/claude.json"),
        include_str!("../../../Resources/providers/codex.json"),
        include_str!("../../../Resources/providers/antigravity.json"),
        include_str!("../../../Resources/providers/copilot.json"),
    ] {
        let config = ProviderConfig::parse(text.as_bytes()).unwrap();
        assert!(!config.id.is_empty());
        assert!(config.interval >= 30);
    }
}

#[test]
fn refresh_intervals_and_invalid_brand_color_follow_mac_defaults() {
    let mut raw: serde_json::Value =
        serde_json::from_str(include_str!("../../../Resources/providers/claude.json")).unwrap();
    raw["refresh"]["intervalSeconds"] = serde_json::json!(1);
    raw["iconColor"] = serde_json::json!("invalid");
    let parsed = ProviderConfig::parse(raw.to_string().as_bytes()).unwrap();
    assert_eq!(parsed.interval, 60);
    assert_eq!(parsed.color, [110, 110, 110]);
}

#[test]
fn invalid_config_and_utf8_bom_are_handled() {
    assert!(ProviderConfig::parse(b"{}").is_err());
    let text = include_str!("../../../Resources/providers/claude.json");
    let bytes = [b"\xef\xbb\xbf".as_slice(), text.as_bytes()].concat();
    assert!(ProviderConfig::parse(&bytes).is_ok());
    let mut config: serde_json::Value = serde_json::from_str(text).unwrap();
    config["source"]["timeoutSeconds"] = serde_json::json!(121);
    assert!(ProviderConfig::parse(config.to_string().as_bytes()).is_err());
}

#[test]
fn arbitrary_request_json_is_not_treated_as_mapping_schema() {
    let s = serde_json::json!({"type":"stdio","executable":"test","steps":[{"write":{"path":"local-file","error":false,"root":"payload"}},{"await":{"capture":"out"}}],"output":"out"});
    assert!(Source::parse(&s).is_ok());
}
