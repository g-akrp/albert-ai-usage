use std::collections::BTreeMap;
fn insert(env: &mut BTreeMap<String, String>, name: &str, value: String) {
    let key = env
        .keys()
        .find(|k| k.eq_ignore_ascii_case(name))
        .cloned()
        .unwrap_or_else(|| name.to_string());
    env.insert(key, value);
}
pub fn environment(extra: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = BTreeMap::new();
    // Read only named, required variables. Never enumerate the credential-bearing environment.
    for name in [
        "PATH",
        "PATHEXT",
        "SystemRoot",
        "SystemDrive",
        "windir",
        "ComSpec",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        "APPDATA",
        "LOCALAPPDATA",
        "ProgramData",
        "TEMP",
        "TMP",
        "USERNAME",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "no_proxy",
        "all_proxy",
    ] {
        if let Ok(value) = std::env::var(name) {
            insert(&mut env, name, value);
        }
    }
    let mut paths = Vec::new();
    for (root, key) in [
        (
            windows::Win32::System::Registry::HKEY_LOCAL_MACHINE,
            "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        ),
        (
            windows::Win32::System::Registry::HKEY_CURRENT_USER,
            "Environment",
        ),
    ] {
        if let Some(value) = registry_string(root, key, "Path") {
            let mut expanded = value;
            for (name, value) in &env {
                expanded = expanded
                    .replace(&format!("%{name}%"), value)
                    .replace(&format!("%{}%", name.to_uppercase()), value);
            }
            paths.push(expanded);
        }
    }
    if let Some(p) = env.get("PATH") {
        paths.push(p.clone());
    }
    if let Some(home) = env.get("USERPROFILE") {
        paths.push(format!(
            "{home}\\.local\\bin;{home}\\AppData\\Local\\agy\\bin"
        ));
        env.insert("HOME".into(), home.clone());
    }
    if let Some(appdata) = env.get("APPDATA") {
        paths.push(format!("{appdata}\\npm"));
    }
    env.insert("PATH".into(), paths.join(";"));
    env.insert("TERM".into(), "dumb".into());
    env.insert("NO_COLOR".into(), "1".into());
    for (key, value) in extra {
        if !["GH_TOKEN", "GITHUB_TOKEN"].contains(&key.to_uppercase().as_str()) {
            let value = if let Some(rest) = value.strip_prefix("~/") {
                format!(
                    "{}\\{rest}",
                    env.get("USERPROFILE").map(String::as_str).unwrap_or("")
                )
            } else {
                value.clone()
            };
            insert(&mut env, key, value);
        }
    }
    env
}
pub fn registry_string(
    root: windows::Win32::System::Registry::HKEY,
    key: &str,
    name: &str,
) -> Option<String> {
    use windows::{core::PCWSTR, Win32::System::Registry::*};
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut bytes = 0;
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND;
    unsafe {
        if RegGetValueW(
            root,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            flags,
            None,
            None,
            Some(&mut bytes),
        )
        .is_err()
        {
            return None;
        }
    }
    if bytes > 65536 {
        return None;
    }
    let mut data = vec![0u16; (bytes as usize).div_ceil(2)];
    unsafe {
        if RegGetValueW(
            root,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            flags,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
        .is_err()
        {
            return None;
        }
    }
    Some(String::from_utf16_lossy(
        &data[..data.iter().position(|c| *c == 0).unwrap_or(data.len())],
    ))
}
