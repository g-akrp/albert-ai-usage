use ai_usage_core::{
    card_model::{Run, Tone},
    panel_layout::Rect,
    pixel_icon,
};
use std::{ffi::c_void, path::Path};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::*,
        Globalization::*,
        Graphics::Gdi::*,
        System::{Registry::*, Time::*},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn dark() -> bool {
    dword(
        HKEY_CURRENT_USER,
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        "AppsUseLightTheme",
    ) == Some(0)
}
fn dword(root: HKEY, key: &str, name: &str) -> Option<u32> {
    let k = wide(key);
    let n = wide(name);
    let mut value = 0u32;
    let mut size = 4;
    unsafe {
        if RegGetValueW(
            root,
            PCWSTR(k.as_ptr()),
            PCWSTR(n.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
        .is_err()
        {
            return None;
        }
    }
    Some(value)
}
pub fn login_enabled() -> bool {
    ai_usage_core::child_env::registry_string(
        HKEY_CURRENT_USER,
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "AI Usage",
    )
    .is_some()
}
pub fn set_login(enabled: bool) -> windows::core::Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()?;
        let result = if enabled {
            let path = std::env::current_exe().map_err(|_| windows::core::Error::from_thread())?;
            let value = wide(&format!("\"{}\"", path.display()));
            let bytes = std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * 2);
            RegSetValueExW(key, w!("AI Usage"), None, REG_SZ, Some(bytes)).ok()
        } else {
            let result = RegDeleteValueW(key, w!("AI Usage"));
            if result == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                result.ok()
            }
        };
        let _ = RegCloseKey(key);
        result
    }
}
pub fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    let path = wide(&path.to_string_lossy());
    unsafe {
        let _ = ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}
pub fn format_date(epoch: i64) -> String {
    unsafe {
        let Some(ticks) = epoch
            .checked_add(11644473600)
            .and_then(|s| s.checked_mul(10000000))
            .filter(|s| *s >= 0)
        else {
            return "Unknown reset".into();
        };
        let file = FILETIME {
            dwLowDateTime: ticks as u32,
            dwHighDateTime: (ticks as u64 >> 32) as u32,
        };
        let mut utc = SYSTEMTIME::default();
        let mut local = SYSTEMTIME::default();
        if FileTimeToSystemTime(&file, &mut utc).is_err()
            || SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).is_err()
        {
            return "Unknown reset".into();
        }
        let mut date = [0u16; 64];
        let mut time = [0u16; 64];
        GetDateFormatEx(
            PCWSTR::null(),
            ENUM_DATE_FORMATS_FLAGS(0),
            Some(&local),
            w!("MMM d"),
            Some(&mut date),
            PCWSTR::null(),
        );
        GetTimeFormatEx(
            PCWSTR::null(),
            TIME_FORMAT_FLAGS(0),
            Some(&local),
            w!("h:mm tt"),
            Some(&mut time),
        );
        format!("{}, {}", from_buf(&date), from_buf(&time))
    }
}
pub fn format_time(epoch: i64) -> String {
    format_date(epoch)
        .split_once(", ")
        .map(|(_, s)| s.to_string())
        .unwrap_or_else(|| "--".into())
}
fn from_buf(v: &[u16]) -> String {
    String::from_utf16_lossy(&v[..v.iter().position(|c| *c == 0).unwrap_or(v.len())])
}
pub struct Tray {
    hwnd: HWND,
    icon: Option<HICON>,
    pub message: u32,
}
impl Tray {
    pub fn new(hwnd: HWND, message: u32) -> Self {
        Self {
            hwnd,
            icon: None,
            message,
        }
    }
    pub fn update(
        &mut self,
        runs: &[Run],
        pin: Option<&str>,
        scale: f32,
        add: bool,
    ) -> windows::core::Result<()> {
        unsafe {
            let selected = pin.and_then(|p| {
                let mut s = p.splitn(2, '|');
                let id = s.next()?;
                Some((runs.iter().find(|r| r.id == id)?, s.next()))
            });
            let mut label = "AI".to_string();
            let mut value = "--".to_string();
            let mut tone = Tone::Secondary;
            let mut brand = [140, 140, 140];
            let mut tip = "AI Usage: all providers are off".to_string();
            if let Some((run, meter)) = selected {
                label = run.label.clone();
                brand = run.color;
                tip = run.name.clone();
                match &run.result {
                    None => {
                        tip.push_str(": loading");
                    }
                    Some(Err(e)) => {
                        value = "ERR".into();
                        tone = Tone::Red;
                        tip.push_str(": ");
                        tip.push_str(e);
                    }
                    Some(Ok(report)) => {
                        let (headline, maximum) = if let Some(meter) = meter {
                            if let Some(m) = report.meters.iter().find(|m| m.id == meter) {
                                label = m.label.chars().take(3).collect::<String>().to_uppercase();
                                let r = ai_usage_core::report::Report {
                                    plan: None,
                                    available: None,
                                    access: None,
                                    meters: vec![m.clone()],
                                };
                                (r.headline(), r.max_percent())
                            } else {
                                (report.headline(), report.max_percent())
                            }
                        } else {
                            (report.headline(), report.max_percent())
                        };
                        value = headline
                            .map(|n| format!("{n:.0}%"))
                            .unwrap_or_else(|| "--".into());
                        tone = Tone::of(maximum);
                        tip.push_str(&format!(": {value}"));
                    }
                }
            }
            let size = (16.0 * scale).round().clamp(16.0, 64.0) as usize;
            let pixels = pixel_icon::render(&label, &value, brand, tone.rgb(), size);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size as i32,
                    biHeight: -(size as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast(), pixels.len());
            let mask_data = vec![0u8; size.div_ceil(32) * 4 * size];
            let mask = CreateBitmap(
                size as i32,
                size as i32,
                1,
                1,
                Some(mask_data.as_ptr().cast::<c_void>()),
            );
            let result = CreateIconIndirect(&ICONINFO {
                fIcon: true.into(),
                hbmMask: mask,
                hbmColor: bitmap,
                ..Default::default()
            });
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteObject(mask.into());
            let icon = result?;
            let mut data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: self.message,
                hIcon: icon,
                ..Default::default()
            };
            let tip: Vec<u16> = tip.encode_utf16().take(127).collect();
            data.szTip[..tip.len()].copy_from_slice(&tip);
            let ok = Shell_NotifyIconW(if add { NIM_ADD } else { NIM_MODIFY }, &data).as_bool();
            if !ok && !add {
                let _ = Shell_NotifyIconW(NIM_ADD, &data);
            }
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
            if let Some(old) = self.icon.replace(icon) {
                let _ = DestroyIcon(old);
            }
            Ok(())
        }
    }
    pub fn anchor(&self) -> Rect {
        unsafe {
            if let Ok(rect) = Shell_NotifyIconGetRect(&NOTIFYICONIDENTIFIER {
                cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                ..Default::default()
            }) {
                return Rect {
                    x: rect.left as f32,
                    y: rect.top as f32,
                    w: (rect.right - rect.left) as f32,
                    h: (rect.bottom - rect.top) as f32,
                };
            }
            let mut p = POINT::default();
            let _ = GetCursorPos(&mut p);
            Rect {
                x: p.x as f32,
                y: p.y as f32,
                w: 1.0,
                h: 1.0,
            }
        }
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            let data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            if let Some(icon) = self.icon.take() {
                let _ = DestroyIcon(icon);
            }
        }
    }
}
