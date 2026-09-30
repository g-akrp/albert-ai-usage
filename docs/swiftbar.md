---
title: SwiftBar plugin
description: First visual interface -- a menu-bar plugin, no native GUI code needed.
date: 2026-09-30
---

# SwiftBar plugin

The first visual interface. [SwiftBar](https://github.com/swiftbar/SwiftBar) is a third-party macOS menu-bar host that runs a script and renders its stdout — using it means zero native Swift/AppKit code for V0.1.0's visualization. Verified against SwiftBar's own plugin API docs, not guessed.

## Setup

1. Install SwiftBar (already on this machine, via `brew install --cask swiftbar`, if not: same command).
2. Build the release binary: `make build` (or `cargo build --manifest-path core/Cargo.toml --release`).
3. In SwiftBar's Plugin Folder, symlink `swiftbar/albert-usage.30s.sh`:
   ```bash
   ln -s /path/to/albert-ai-usage/swiftbar/albert-usage.30s.sh ~/YourSwiftBarPluginFolder/
   ```
4. Refresh SwiftBar (or wait for the 30s interval in the filename).

## How it works

- `swiftbar/albert-usage.30s.sh` is a thin wrapper (resolves paths, sets `ALBERT_PROVIDERS_DIR`, execs the real binary with `--swiftbar`). All logic lives in `core/src/swiftbar.rs`, unit-tested independent of SwiftBar itself.
- Rename the file to change the refresh interval (`{name}.{time}.{ext}`, e.g. `.1m.sh`, `.5m.sh` -- SwiftBar's own naming convention).

## Pinning

Click any provider's row in the dropdown to pin it — that's what shows compactly in the menu bar (`★`/`☆` marks which one). Nothing is pinned by default: the menu bar cycles every provider's headline number instead.

Mechanism: clicking calls `bash=<this binary> param1=--pin param2=<id> terminal=false refresh=true` (SwiftBar's own click-action syntax) — writes the id to `~/.config/albert-ai-usage/pinned`, then SwiftBar re-runs the plugin immediately (`refresh=true`).

## Compact number shown

The highest used-percent across every window in the pinned provider's report (the most urgent number is the most useful at a glance) — e.g. "Codex 47%", colored green/orange/red at 70%/90% thresholds. A stale pin (a provider id no longer in `providers/`) falls back to the cycling default instead of erroring.

## Parallel refresh

Providers run concurrently, one thread each (`core/src/parallel.rs`), so a refresh takes as long as the slowest provider instead of the sum. Output order stays sorted by config filename.

## Error icon

A provider whose fetch fails shows the normal two-row icon with its brand label on top and a red `ERR` below, instead of disappearing from the menu bar. Errored providers take part in the cycling header, and a pinned provider that errors shows its error icon. The reason is in the icon's `alt=` text and in the dropdown row.
