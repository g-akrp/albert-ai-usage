use ai_usage_core::store::load;

#[test]
fn shipped_windows_overrides_load_all_providers() {
    let (configs, errors) = load(None);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        configs.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        vec!["claude", "codex", "antigravity", "copilot"]
    );
    assert_eq!(configs[3].source.executable, "powershell.exe");
    assert!(configs[3].source.args.iter().all(|a| a != "Bypass"));
    assert!(configs[1].source.raw["steps"][2]["write"]["params"].is_null());
}
