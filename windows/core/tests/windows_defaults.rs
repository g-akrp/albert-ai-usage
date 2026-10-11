use ai_usage_core::settings_model::Settings;

#[test]
fn new_windows_settings_enable_only_claude_and_codex() {
    for settings in [
        Settings::default(),
        Settings::parse(b""),
        Settings::parse(b"{}"),
    ] {
        assert_eq!(settings.disabled, vec!["antigravity", "copilot"]);
    }
    // Explicit preferences, including enabling every provider, remain authoritative.
    assert!(Settings::parse(br#"{"disabledProviders":[]}"#)
        .disabled
        .is_empty());
}
