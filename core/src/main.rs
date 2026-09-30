use std::fs;
use std::path::PathBuf;

use albert_ai_usage_core::config::{load_config, run_provider};
use albert_ai_usage_core::{format_report, icon, swiftbar};

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
            let icon_label = config
                .icon_label
                .clone()
                .unwrap_or_else(|| config.id.chars().take(3).collect::<String>().to_uppercase());
            let icon_color = config
                .icon_color
                .as_deref()
                .and_then(icon::parse_hex_color)
                .unwrap_or([110, 110, 110]); // neutral gray fallback
            Some(swiftbar::ProviderRun {
                id: config.id,
                name: config.name,
                icon_label,
                icon_color,
                result,
            })
        })
        .collect()
}

/// Renders each provider's real, live icon at a larger preview scale
/// (8x instead of the 2x used for the actual menu bar) and saves it
/// as a PNG so it can be looked at directly -- `make icon-preview`.
fn icon_preview() {
    let dir = std::env::var("ALBERT_ICON_PREVIEW_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/albert-icon-preview"));
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("could not create {dir:?}: {e}");
        std::process::exit(1);
    }

    for run in run_all_providers() {
        let Ok(report) = run.result else {
            eprintln!("{}: skipped (provider errored)", run.id);
            continue;
        };
        let Some(pct) = swiftbar::max_percent(&report) else {
            eprintln!("{}: skipped (no numeric usage)", run.id);
            continue;
        };
        let pct_text = format!("{pct:.0}%");
        let png = icon::render_two_line_png(
            &run.icon_label,
            &pct_text,
            run.icon_color,
            icon::severity_rgb(pct),
            8,
        );
        let path = dir.join(format!("{}.png", run.id));
        if let Err(e) = fs::write(&path, &png) {
            eprintln!("{}: could not write {path:?}: {e}", run.id);
            continue;
        }
        println!("{path:?}");
    }
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

    if args.iter().any(|a| a == "--icon-preview") {
        icon_preview();
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

    for run in run_all_providers() {
        println!("{}", format_report(&run.id, &run.name, &run.result));
    }
}
