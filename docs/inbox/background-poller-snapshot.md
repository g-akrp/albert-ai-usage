# Background Poller and Atomic Snapshot Cache

Status: proposed
Date: 2026-10-02

## Problem
Currently, the SwiftBar menu-bar wrapper synchronously executes `albert-usage --swiftbar`, running provider CLI commands in parallel on each 30-second interval or user dropdown click. When external coding-agent CLIs (Claude, Codex, Copilot, Antigravity) take multiple seconds to respond, SwiftBar hangs or displays a spinner in the macOS menu bar. Additionally, clicking a provider to pin it triggers a full synchronous re-fetch of every provider before updating the icon, creating avoidable user-facing latency.

## Proposal
Decouple provider data retrieval from UI rendering with a file-based cache that the reader refreshes itself (stale-while-revalidate). No long-running process is required.

- **Snapshot (data, not rendered output):** `~/.config/albert-ai-usage/snapshot.json` holds normalized provider data, not menu-bar text or icons, so pin changes re-render from cache. Fields:
  - `version` — schema version. If the version is unknown or the file cannot be parsed, the snapshot is treated as missing.
  - `updated_at` — time of the last completed fetch.
  - `poll_interval_secs` — interval the writer used. Staleness is derived from it.
  - per provider: `fetched_at`, the latest result (report or error), and the last successful report, so a transient error does not erase good data.
- **Fetch (`albert-usage --fetch`):** one-shot. Takes an exclusive lock (`fetch.lock`, non-blocking; exits immediately if another fetch holds it), queries providers in parallel (per-provider `timeoutSeconds` already bounds a hung CLI), and atomically replaces the snapshot. The atomic write uses a unique temp file in the same directory, fsync, then `rename(2)`. The file is created with mode `0600`.
- **Render (`albert-usage --swiftbar`):** reads the snapshot and pin state and renders immediately without waiting on provider CLIs.
  - If no snapshot exists: fetch synchronously (first run only).
  - If the snapshot is older than `poll_interval_secs`: render the cached data, then spawn one detached `--fetch` and exit. The fetch lock guarantees at most one in-flight fetch. SwiftBar's next scheduled run shows the fresh data.
- **Staleness display:** the dropdown always shows the data age ("Updated 2m ago"). If age exceeds 3 × `poll_interval_secs`, a warning row is shown (`⚠️ Data stale`), plus the per-provider age for providers whose last fetch failed and are showing last-good data.
- **Refresh now:** a dropdown row runs `--fetch` with `refresh=true` for a manual, user-initiated update.
- **Instant pinning:** `--pin` writes the pin file, and SwiftBar's `refresh=true` re-run renders from cache.
- **Daemon (`albert-usage --daemon`, optional):** a loop calling the same fetch path every `ALBERT_POLL_INTERVAL_SECS` (default 30). Not needed for SwiftBar correctness; useful for warm data and for future native shells. Errors in one iteration (including an unreadable providers directory) are logged and the loop continues.

## Scope
- **In:**
  - Snapshot schema (versioned), atomic write, read, and staleness logic in the shared Rust core.
  - Fetch lock (single-flight) shared by `--fetch`, `--daemon`, and the detached refresh.
  - `--fetch` one-shot command and the optional `--daemon` loop.
  - Stale-while-revalidate in `--swiftbar`, including the detached refresh spawn and first-run synchronous fallback.
  - Data age, stale warning, last-good fallback, and "Refresh now" in dropdown rendering.
  - Measured render latency for the cached path (target: under 50 ms including icon rasterization; record the measured number in the spec).
- **Out:**
  - Desktop UI implementations (macOS Swift/AppKit and Windows C#/WinUI remain deferred).
  - Cross-platform GUI frameworks (Electron, Tauri, Wails).
  - OS service registration or launch-at-login for `--daemon` (deferred per `AGENTS.md`).

## Alternatives considered
- **Status quo (synchronous fetch per render):** Simple, but freezes the macOS menu bar during slow provider responses and makes pinning sluggish.
- **Required background daemon as the only producer:** Without OS launch-at-login (deferred), nothing starts it. Once the first snapshot exists the reader would show stale data indefinitely, which is worse than the status quo. Rejected as the primary path; kept as an optional accelerator.
- **In-memory IPC daemon (UNIX domain socket / named pipe):** Requires platform-specific socket lifecycle management, daemon discovery, and reconnection logic. A file-based atomic snapshot gives fast reads with no IPC.
- **SwiftBar-managed detached children without a lock:** Concurrent renders would each spawn a fetch, piling up provider CLI runs and racing on the snapshot file. The fetch lock removes both problems, which is why stale-while-revalidate is acceptable here.
- **Storing rendered output (menu-bar text / icons) in the snapshot:** Pin changes and render tweaks would require a re-fetch. Storing normalized data keeps rendering cheap and shell-independent.

## Impact
- **Rust core:** Adds snapshot schema, atomic I/O, fetch lock, staleness rules, refresh spawn, and `--fetch` / `--daemon` CLI options. Business logic remains in the core.
- **Swift/AppKit shell (macOS):** When native shell work begins, it can read the same snapshot (or call into the core) instead of running provider CLIs on the UI thread.
- **C#/WinUI shell (Windows):** Same as macOS. The spawn and lock code must stay portable (file lock via a cross-platform crate, no `flock`-only assumptions).
- `docs/core-abi.md`: No ABI change now. If native shells need snapshot access through the ABI, the exported functions are added there first.
- `docs/architecture.md`: Add the snapshot cache, fetch lock, and the stale-while-revalidate flow.
- `docs/swiftbar.md`: Describe the cached render, data age, "Refresh now", and the optional daemon.

## Open questions
- Running `--daemon` automatically at login (macOS `launchd` LaunchAgent vs Windows Startup task / service) is an OS-specific lifecycle decision that needs separate approval per `AGENTS.md`. This proposal works without it.
- Should the stale-warning threshold (3 × interval) be configurable, or is the derived value enough?
- Should the snapshot live in a cache directory (`~/Library/Caches`, `%LOCALAPPDATA%`) instead of `~/.config`, since it is regenerable? Changing it later is cheap.

## Note
A spike implementation (`core/src/snapshot.rs`, `--daemon`, `--fetch`, `make daemon` / `make fetch`) exists in the working tree ahead of approval. After approval it will be revised to match this proposal (unique temp file, lock, versioned schema, last-good data, daemon error handling, reader-triggered refresh).
