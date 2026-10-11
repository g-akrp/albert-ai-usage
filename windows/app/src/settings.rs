use ai_usage_core::settings_model::Settings;
use std::path::{Path, PathBuf};
use windows::{
    core::PCWSTR,
    Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
};
pub fn folder(demo: bool) -> PathBuf {
    if demo {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("ai-usage/demo")
    } else {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("ai-usage")
    }
}
pub fn load(folder: &Path) -> Settings {
    let path = folder.join("settings.json");
    let bytes = std::fs::metadata(&path)
        .ok()
        .filter(|m| m.len() <= 65536)
        .and_then(|_| std::fs::read(&path).ok())
        .unwrap_or_default();
    Settings::parse(&bytes)
}
pub fn save(folder: &Path, settings: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|_| "could not create settings folder")?;
    let temporary = folder.join(format!("settings-{}.tmp", std::process::id()));
    let final_path = folder.join("settings.json");
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary).map_err(|_| "could not write settings")?;
        file.write_all(settings.to_json().as_bytes())
            .map_err(|_| "could not write settings")?;
        file.sync_all().map_err(|_| "could not sync settings")?;
    }
    let temporary = crate::platform::wide(&temporary.to_string_lossy());
    let final_path = crate::platform::wide(&final_path.to_string_lossy());
    unsafe {
        MoveFileExW(
            PCWSTR(temporary.as_ptr()),
            PCWSTR(final_path.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|_| "could not replace settings".to_string())
    }
}
