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
        &Default::default(),
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

fn status_layout(
    run: &Run,
    status: &ai_usage_core::panel_layout::Status,
) -> ai_usage_core::panel_layout::Layout {
    layout(
        &[Card::from_run(run)],
        &Settings::default(),
        &[],
        status,
        "3:21 PM",
        "0.1.0",
        false,
        false,
        700.0,
        |s, size| s.len() as f32 * size * 0.55,
        |_| "reset".into(),
    )
}

#[test]
fn busy_provider_shows_spinner_and_refreshing_footer_without_hit() {
    let (providers, _) = store::load(None);
    let run = Run::loading(&providers[0]);
    let mut status = ai_usage_core::panel_layout::Status::default();
    let idle = status_layout(&run, &status);
    assert!(idle.spinners.is_empty());
    assert!(idle
        .hits
        .iter()
        .any(|h| matches!(h.action, Action::Refresh)));
    status.busy.insert(run.config_id.clone());
    let busy = status_layout(&run, &status);
    assert_eq!(busy.spinners.len(), 1);
    assert!(busy.texts.iter().any(|t| t.text == "Refreshing…"));
    assert!(!busy
        .hits
        .iter()
        .any(|h| matches!(h.action, Action::Refresh)));
    assert!(!busy.texts.iter().any(|t| t.text == "Refresh Now"));
}

#[test]
fn error_card_has_retry_and_backoff_line_only_when_failed() {
    let (providers, _) = store::load(None);
    let mut run = Run::loading(&providers[0]);
    let mut status = ai_usage_core::panel_layout::Status::default();
    status.retry_in.insert(run.config_id.clone(), 240);
    let loading = status_layout(&run, &status);
    assert!(!loading
        .hits
        .iter()
        .any(|h| matches!(h.action, Action::Retry(_))));
    run.result = Some(Err("claude timed out".into()));
    let failed = status_layout(&run, &status);
    assert!(failed
        .hits
        .iter()
        .any(|h| matches!(&h.action, Action::Retry(id) if *id == run.config_id)));
    assert!(failed.texts.iter().any(|t| t.text == "Retrying in 4m"));
    status.busy.insert(run.config_id.clone());
    let retrying = status_layout(&run, &status);
    assert!(!retrying
        .hits
        .iter()
        .any(|h| matches!(h.action, Action::Retry(_))));
    assert!(!retrying
        .texts
        .iter()
        .any(|t| t.text.starts_with("Retrying in")));
}

#[test]
fn updated_text_index_points_at_updated_line_and_rings_have_keys() {
    let (providers, _) = store::load(None);
    let run = Run::loading(&providers[0]);
    let l = status_layout(&run, &Default::default());
    let i = l.updated.expect("updated index");
    assert_eq!(l.texts[i].text, "Updated 3:21 PM");
    assert!(l.rings.iter().all(|r| !r.key.is_empty()));
}

#[test]
fn due_retry_says_soon() {
    let (providers, _) = store::load(None);
    let mut run = Run::loading(&providers[0]);
    run.result = Some(Err("x".into()));
    let mut status = ai_usage_core::panel_layout::Status::default();
    status.retry_in.insert(run.config_id.clone(), 0);
    let l = status_layout(&run, &status);
    assert!(l.texts.iter().any(|t| t.text == "Retrying soon"));
}
