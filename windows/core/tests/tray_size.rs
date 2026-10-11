#[test]
fn tray_text_fills_available_height() {
    let pixels = ai_usage_core::pixel_icon::render("CLD", "83", [255; 3], [255; 3], 16);
    assert!(pixels[..16 * 4].chunks_exact(4).any(|p| p[3] != 0));
    assert!(pixels[15 * 16 * 4..].chunks_exact(4).any(|p| p[3] != 0));
}
