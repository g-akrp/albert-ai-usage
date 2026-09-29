---
title: Repository Agent Instructions
description: Scope and architecture constraints for work in this repository.
---

# Repository instructions

- Treat `docs/architecture.md` as the approved initial product architecture and `docs/core-abi.md` as the single source of truth for the Rust/shell ABI.
- Keep business logic in the shared Rust core; platform UI must remain native Swift/AppKit on macOS and C#/WinUI on Windows.
- Do not introduce Electron, Tauri, Wails, or another cross-platform GUI framework.
- Keep OS-specific credential storage, tray/notification-area behavior, and launch-at-login unresolved until separately approved.
- Do not hand-copy FFI bindings; follow the generation and ownership rules in `docs/core-abi.md`.
