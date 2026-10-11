use ai_usage_core::{
    card_model::{Card, Run},
    panel_layout::{layout, position, Action, Rect},
    settings_model::Settings,
    store,
};

#[test]
fn flyout_is_clamped_on_negative_coordinate_monitors() {
    let work = Rect {
        x: -1920.0,
        y: -400.0,
        w: 1920.0,
        h: 1080.0,
    };
    let p = position(
        Rect {
            x: -12.0,
            y: 660.0,
            w: 20.0,
            h: 20.0,
        },
        work,
        450.0,
        1000.0,
    );
    assert!(p.x >= work.x && p.y >= work.y);
    assert!(p.x + p.w <= work.x + work.w && p.y + p.h <= work.y + work.h);
}

#[test]
fn collapse_keeps_message_and_has_explicit_hit_target() {
    let (providers, _) = store::load(None);
    let run = Run::loading(&providers[0]);
    let card = Card::from_run(&run);
    let settings = Settings {
        collapsed: vec![run.id.clone()],
        ..Default::default()
    };
    let l = layout(
        &[card],
        &settings,
        &[],
        "3:21 PM",
        "0.1.0",
        false,
        false,
        700.0,
        |s, size| s.len() as f32 * size * 0.55,
        |_| "reset".into(),
    );
    assert!(l
        .hits
        .iter()
        .any(|h| matches!(h.action, Action::Collapse(_))));
    assert!(l.texts.iter().any(|t| t.text == "Loading…"));
    assert!(l.rings.is_empty());
}
