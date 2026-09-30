---
title: Albert AI Usage
description: Native desktop usage and quota monitor for AI coding agents.
---

# Albert AI Usage

Albert AI Usage is a planned native desktop application for viewing usage and quota information from AI coding-agent providers on macOS and Windows.

The shared Rust core handles provider polling, JSON parsing, proxy fallback, and provider registration. Native Swift/AppKit and C#/WinUI shells provide the platform-specific user interfaces. See [the architecture specification](docs/architecture.md) and [the core ABI contract](docs/core-abi.md).

## Repository layout

- `core/` — shared Rust library and C ABI.
- `providers/` — JSON provider configs (Claude, Codex, Copilot, Antigravity).
- `swiftbar/` — SwiftBar plugin wrapper, the current macOS interface.
- `macos/` — native Swift/AppKit shell (planned).
- `windows/` — native C#/WinUI shell (planned).
- `docs/` — architecture and interface specifications.

## Current state: SwiftBar menu bar

Until the native shells exist, usage shows in the macOS menu bar through [SwiftBar](https://github.com/swiftbar/SwiftBar). See [the SwiftBar doc](docs/swiftbar.md).

- Two-row icon per provider: brand-colored label on top, usage percent below (green, orange, or red at 70% and 90%).
- Providers refresh in parallel, so a refresh takes as long as the slowest one.
- A provider that fails shows its label with a red `ERR` instead of vanishing. The reason is in the dropdown.
- Click a provider in the dropdown to pin it. With nothing pinned, the icon cycles through all providers.

```bash
make build            # release binary
make swiftbar-install # symlink the plugin into SwiftBar's plugin folder
make run              # print every provider's usage in the terminal
make icon-preview     # render icons to /tmp/albert-icon-preview
make check            # fmt, clippy, tests
```

No cross-platform GUI framework is part of this design.
