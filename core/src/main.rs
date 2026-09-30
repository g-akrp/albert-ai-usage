use std::fs;
use std::path::PathBuf;

use albert_ai_usage_core::config::{load_config, run_provider};
use albert_ai_usage_core::{format_report, swiftbar};

fn providers_dir() -> PathBuf {
    std::env::var("ALBERT_PROVIDERS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("providers"))
}

/// Where the pinned provider id is stored. `ALBERT_STATE_DIR` for
/// tests/overrides; otherwise `~/.config/albert-ai-usage/pinned`.
fn pinned_path() -> PathBuf {
    let dir = std::env::var("ALBERT_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config/albert-ai-usage")
        });
    dir.join("pinned")
}

fn read_pinned() -> Option<String> {
    fs::read_to_string(pinned_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_pinned(id: &str) -> std::io::Result<()> {
    let path = pinned_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, id)
}

fn run_all_providers() -> Vec<swiftbar::ProviderRun> {
    let dir = providers_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("could not read providers directory {dir:?}: {e}");
            std::process::exit(1);
        }
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .filter_map(|path| {
            let json = fs::read_to_string(&path).ok()?;
            let config = load_config(&json).ok()?;
            let result = run_provider(&config);
            Some((config.id, config.name, result))
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Some(id) = args
        .iter()
        .position(|a| a == "--pin")
        .and_then(|i| args.get(i + 1))
    {
        if let Err(e) = write_pinned(id) {
            eprintln!("could not save pinned provider: {e}");
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--swiftbar") {
        let binary_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_else(|| args[0].clone());
        let runs = run_all_providers();
        print!(
            "{}",
            swiftbar::render(&binary_path, read_pinned().as_deref(), &runs)
        );
        return;
    }

    for (id, name, result) in run_all_providers() {
        println!("{}", format_report(&id, &name, &result));
    }
}
