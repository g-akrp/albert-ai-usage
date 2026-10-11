use std::{collections::BTreeMap, path::PathBuf};
pub fn find(name: &str, env: &BTreeMap<String, String>) -> Option<PathBuf> {
    let explicit = name.contains('/') || name.contains('\\');
    let name = if let Some(rest) = name.strip_prefix("~/") {
        format!("{}\\{rest}", env.get("USERPROFILE")?)
    } else {
        name.to_string()
    };
    let extensions = env
        .get("PATHEXT")
        .map(String::as_str)
        .unwrap_or(".COM;.EXE;.BAT;.CMD");
    // npm installs extensionless POSIX scripts beside Windows .cmd shims.
    // Only explicitly named extensions are launched verbatim.
    let suffixes: Vec<&str> = if std::path::Path::new(&name).extension().is_some() {
        vec![""]
    } else {
        extensions.split(';').filter(|s| !s.is_empty()).collect()
    };
    let dirs: Vec<PathBuf> = if explicit {
        vec![PathBuf::new()]
    } else {
        env.get("PATH")?
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| PathBuf::from(s.trim_matches('"')))
            .collect()
    };
    for dir in dirs {
        for suffix in &suffixes {
            let file = dir.join(format!("{name}{suffix}"));
            if file.is_file() {
                return Some(file);
            }
        }
    }
    None
}
pub fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    let mut slashes = 0;
    for c in value.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        if c == '"' {
            output.push_str(&"\\".repeat(slashes * 2 + 1));
        } else {
            output.push_str(&"\\".repeat(slashes));
        }
        slashes = 0;
        output.push(c);
    }
    output.push_str(&"\\".repeat(slashes * 2));
    output.push('"');
    output
}
