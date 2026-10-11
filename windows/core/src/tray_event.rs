use windows::Win32::UI::Shell::NIN_SELECT;
// Windows SDK defines NIN_KEYSELECT as NIN_SELECT | NINF_KEY (1).
pub const KEY_SELECT: u32 = NIN_SELECT | 1;

/// Version 4 tray icons emit select notifications in addition to mouse events.
pub fn activates_panel(lparam: isize) -> bool {
    matches!((lparam as u32) & 0xffff, NIN_SELECT | KEY_SELECT)
}
