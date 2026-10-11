use ai_usage_core::{
    card_model::Tone,
    card_order::{default_order, move_visible},
    settings_model::Settings,
};

#[test]
fn hidden_cards_keep_slots_when_moving_visible_neighbors() {
    let mut order = vec![
        "claude".into(),
        "codex".into(),
        "antigravity".into(),
        "copilot".into(),
    ];
    move_visible(&mut order, "claude", 1, &["codex".into()]);
    assert_eq!(order, vec!["antigravity", "codex", "claude", "copilot"]);
    assert_eq!(
        default_order(&[
            "copilot".into(),
            "codex".into(),
            "claude".into(),
            "antigravity".into()
        ]),
        vec!["claude", "codex", "antigravity", "copilot"]
    );
}

#[test]
fn settings_recover_and_round_trip_all_ui_choices() {
    assert_eq!(Settings::parse(b"{invalid").collapsed.len(), 0);
    let settings = Settings {
        pin: Some("claude".into()),
        hidden: vec!["copilot:sample".into()],
        collapsed: vec!["codex".into()],
        order: vec!["codex".into(), "claude".into()],
        ..Default::default()
    };
    assert_eq!(Settings::parse(settings.to_json().as_bytes()), settings);
}

#[test]
fn warning_boundaries_are_exact() {
    assert_eq!(Tone::of(Some(69.0)), Tone::Green);
    assert_eq!(Tone::of(Some(70.0)), Tone::Orange);
    assert_eq!(Tone::of(Some(89.0)), Tone::Orange);
    assert_eq!(Tone::of(Some(90.0)), Tone::Red);
}

#[test]
fn grouped_providers_cannot_remain_pinned_as_a_whole() {
    use ai_usage_core::{
        card_model::{ensure_pin, Run},
        report::{Meter, Report, Window},
        store,
    };
    let (configs, _) = store::load(None);
    let c = configs.iter().find(|c| c.id == "antigravity").unwrap();
    let mut run = Run::loading(c);
    let window = Window {
        id: "s".into(),
        label: Some("Session".into()),
        used: Some(10.0),
        resets_at: None,
        duration: Some(18000),
        count: None,
    };
    run.result = Some(Ok(Report {
        plan: None,
        available: None,
        access: None,
        meters: vec![
            Meter {
                id: "Gemini".into(),
                label: "Gemini".into(),
                windows: vec![window.clone()],
            },
            Meter {
                id: "Claude".into(),
                label: "Claude".into(),
                windows: vec![window],
            },
        ],
    }));
    let mut settings = Settings {
        pin: Some("antigravity".into()),
        ..Default::default()
    };
    ensure_pin(&mut settings, &[run]);
    assert_eq!(settings.pin.as_deref(), Some("antigravity|Gemini"));
}

#[test]
fn legacy_copilot_pin_migrates_to_first_account() {
    use ai_usage_core::{
        card_model::{ensure_pin, Run},
        store,
    };
    let (configs, _) = store::load(None);
    let claude = Run::loading(&configs[0]);
    let mut copilot = Run::loading(&configs[3]);
    copilot.id = "copilot:sample".into();
    copilot.account = Some("sample".into());
    let mut settings = Settings {
        pin: Some("copilot".into()),
        ..Default::default()
    };
    ensure_pin(&mut settings, &[claude, copilot]);
    assert_eq!(settings.pin.as_deref(), Some("copilot:sample"));
}
