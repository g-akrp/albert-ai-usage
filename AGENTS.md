---
title: Repository Agent Instructions
description: Scope and architecture constraints for work in this repository.
---

# Repository instructions

- This is one native macOS app: Swift, AppKit only. No SwiftUI, no third-party packages, no cross-platform GUI framework, no helper daemons. The old Rust core and SwiftBar plugin are in git history; do not bring them back.
- Keep memory low: the app's physical footprint is about 11 MB. Check with `footprint -p $(pgrep -x AlbertAIUsage) | grep phys_footprint` after a change that could affect it; stay under 30 MB. No polling beyond the 30 s schedule timer and the 5 s icon cycle.
- Providers are data (`Resources/providers/*.json`, format in `example/AGENTS.md`). Add or fix a provider by editing JSON, not Swift, unless the format itself needs to change.
- Logic belongs in `Sources/AlbertUsageCore/` and is covered by `swift run CoreChecks` (the test suite; XCTest does not build here). Write the check first.
- Never read or store credentials. Providers use their own CLI's login; never pass `GITHUB_TOKEN`/`GH_TOKEN` to them.
- Never edit version numbers by hand; `scripts/release.sh` sets them. Commit, push, and publish only when the human asks.
