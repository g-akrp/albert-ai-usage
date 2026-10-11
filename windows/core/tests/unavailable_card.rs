use ai_usage_core::{
    card_model::{Card, Run},
    report::Report,
    store,
};

#[test]
fn claude_unavailable_usage_has_no_synthetic_percentage() {
    let (configs, _) = store::load(None);
    let mut run = Run::loading(configs.iter().find(|config| config.id == "claude").unwrap());
    run.result = Some(Ok(Report {
        plan: None,
        available: Some(false),
        access: None,
        meters: Vec::new(),
    }));
    let card = Card::from_run(&run);
    assert_eq!(card.message.as_deref(), Some("Usage unavailable"));
    assert!(!card.error);
    assert!(card.notices.is_empty());
    assert!(card.charts.is_empty());
    assert!(card.pills().is_empty());
    assert_eq!(
        run.result.as_ref().unwrap().as_ref().unwrap().headline(),
        None
    );
    for collapsed in [false, true] {
        let mut settings = ai_usage_core::settings_model::Settings::default();
        if collapsed {
            settings.collapsed.push(run.id.clone());
        }
        let layout = ai_usage_core::panel_layout::layout(
            std::slice::from_ref(&card),
            &settings,
            &[],
            "",
            "0.1.0",
            false,
            false,
            600.0,
            |text, size| text.len() as f32 * size / 2.0,
            |_| String::new(),
        );
        assert!(layout
            .texts
            .iter()
            .any(|text| text.text == "Usage unavailable" && text.visibility.shown(None)));
        assert!(layout.rings.is_empty());
        assert!(layout.pills.is_empty());
    }
}
