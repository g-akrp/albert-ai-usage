use crate::{card_order, config::ProviderConfig};
use std::{collections::BTreeMap, path::Path};
pub fn load(user: Option<&Path>) -> (Vec<ProviderConfig>, Vec<String>) {
    let embedded = [
        include_bytes!("../../../Resources/providers/claude.json").as_slice(),
        include_bytes!("../../../Resources/providers/codex.json").as_slice(),
        include_bytes!("../../../Resources/providers/antigravity.json").as_slice(),
        include_bytes!("../../../Resources/providers/copilot.json").as_slice(),
        include_bytes!("../../../Resources/providers-windows/copilot.json").as_slice(),
        include_bytes!("../../../Resources/providers-windows/codex.json").as_slice(),
    ];
    let mut configs = BTreeMap::new();
    let mut errors = Vec::new();
    for bytes in embedded {
        match ProviderConfig::parse(bytes) {
            Ok(c) => {
                configs.insert(c.id.clone(), c);
            }
            Err(e) => errors.push(format!("embedded provider: {e}")),
        }
    }
    if let Some(dir) = user {
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_none_or(|e| e != "json") {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    let result = (|| {
                        let m = std::fs::symlink_metadata(&path).map_err(|_| "unreadable file")?;
                        if !m.file_type().is_file() || m.len() > 262144 {
                            return Err("only regular files up to 256 KiB load".to_string());
                        }
                        let bytes = std::fs::read(&path).map_err(|_| "unreadable file")?;
                        let c = ProviderConfig::parse(&bytes)?;
                        if path.file_stem().and_then(|s| s.to_str()) != Some(c.id.as_str()) {
                            return Err("provider id must match file name".into());
                        }
                        Ok(c)
                    })();
                    match result {
                        Ok(c) => {
                            configs.insert(c.id.clone(), c);
                        }
                        Err(e) => errors.push(format!("{name}: {e}")),
                    }
                }
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    errors.push("could not read user providers folder".into());
                }
            }
        }
    }
    let order = card_order::default_order(&configs.keys().cloned().collect::<Vec<_>>());
    (
        order
            .into_iter()
            .filter_map(|id| configs.remove(&id))
            .collect(),
        errors,
    )
}
