use std::fs;
use std::path::PathBuf;

use albert_ai_usage_core::config::{load_config, run_provider};
use albert_ai_usage_core::format_report;

fn providers_dir() -> PathBuf {
    std::env::var("ALBERT_PROVIDERS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("providers"))
}

fn main() {
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

    for path in paths {
        let json = match fs::read_to_string(&path) {
            Ok(json) => json,
            Err(e) => {
                println!("{path:?} — Error: could not read file ({e})");
                continue;
            }
        };
        let config = match load_config(&json) {
            Ok(config) => config,
            Err(e) => {
                println!("{path:?} — Error: {e}");
                continue;
            }
        };
        let result = run_provider(&config);
        println!("{}", format_report(&config.id, &config.name, &result));
    }
}
