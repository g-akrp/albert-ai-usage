use ai_usage_core::pixel_icon::render;

#[test]
fn longest_tray_value_fits_small_and_hidpi_icons() {
    for size in [16, 20, 24, 32, 48] {
        let pixels = render("CLD", "100%", [217, 119, 87], [255, 59, 48], size);
        assert_eq!(pixels.len(), size * size * 4);
        assert!(pixels.chunks(4).any(|p| p[3] > 0));
        assert!(pixels[..size * 4].chunks_exact(4).any(|p| p[3] > 0));
    }
}
