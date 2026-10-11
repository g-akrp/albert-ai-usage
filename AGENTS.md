---
title: Repository Agent Instructions
description: Scope and architecture constraints for work in this repository.
---

# Repository instructions

- macOS is Swift/AppKit only: no SwiftUI or third-party packages. The approved Windows app is fresh Rust/Win32 under `windows/`, with its own instructions. No cross-platform GUI framework or helper daemon. The old Rust core and SwiftBar plugin are in Git history; do not bring them back.
- Keep memory low: the app's physical footprint is about 11 MB. Check with `footprint -p $(pgrep -x AIUsage) | grep phys_footprint` after a change that could affect it; stay under 30 MB. No polling beyond the 30 s schedule timer.
- Providers are data (`Resources/providers/*.json`, format in `example/AGENTS.md`). Add or fix a provider by editing JSON, not Swift, unless the format itself needs to change.
- macOS logic belongs in `Sources/AIUsageCore/` and is covered by `swift run CoreChecks` (XCTest does not build here). Windows logic belongs in `windows/core/` and is checked with Cargo. Write the check first.
- Never read or store credentials. Providers use their own CLI's login; never pass `GITHUB_TOKEN`/`GH_TOKEN` to them.
- Never edit application version numbers by hand; `scripts/release.sh` sets macOS versions and `windows/scripts/release.ps1` sets Windows versions. Commit, push, and publish only when the human asks.
