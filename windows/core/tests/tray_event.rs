use ai_usage_core::tray_event::{activates_panel, KEY_SELECT};
use windows::Win32::UI::{Shell::NIN_SELECT, WindowsAndMessaging::WM_LBUTTONUP};

#[test]
fn one_mouse_click_opens_panel_once_and_keyboard_selection_works() {
    let mut visible = false;
    for event in [WM_LBUTTONUP, NIN_SELECT] {
        if activates_panel(((1_u32 << 16) | event) as isize) {
            visible = !visible;
        }
    }
    assert!(
        visible,
        "mouse-up followed by select must not close the panel"
    );
    assert!(activates_panel(((1_u32 << 16) | KEY_SELECT) as isize));
}
