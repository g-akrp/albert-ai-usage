---
title: Albert AI Usage
description: Native desktop usage and quota monitor for AI coding agents.
---

# Albert AI Usage

Albert AI Usage is a planned native desktop application for viewing usage and quota information from AI coding-agent providers on macOS and Windows.

The shared Rust core handles provider polling, JSON parsing, proxy fallback, and provider registration. Native Swift/AppKit and C#/WinUI shells provide the platform-specific user interfaces. See [the architecture specification](docs/architecture.md) and [the core ABI contract](docs/core-abi.md).

## Repository layout

- `core/` — shared Rust library and C ABI.
- `macos/` — native Swift/AppKit shell.
- `windows/` — native C#/WinUI shell.
- `docs/` — architecture and interface specifications.

No cross-platform GUI framework is part of this design.
