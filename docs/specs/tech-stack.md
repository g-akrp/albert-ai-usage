# Tech stack

## Windows

The separate Windows implementation uses Rust, Win32, Direct2D, and DirectWrite. Runtime dependencies are `windows` and `serde_json`; `embed-resource` embeds the manifest, icons, and version resource at build time. MSVC builds target x64 and ARM64 with the static CRT. There is no cross-platform GUI framework or helper daemon. Portable ZIPs are the current distribution scope; WiX 7 MSI source is retained but unbuilt pending a future explicit decision. See [Windows release](windows/release.md) for verification limits.

## macOS

- **Language:** Swift 6 tools, Swift 5 language mode.
- **UI:** AppKit only. No SwiftUI, no cross-platform GUI framework.
- **Dependencies:** none. No third-party packages, no helper daemons.
- **Platform:** macOS 13 or later.
- **Build:** Swift Package Manager, Xcode Command Line Tools only. `scripts/build-app.sh` builds, wraps and ad hoc signs the app; `scripts/release.sh` makes the DMG.

## Targets

- `AIUsageCore`: all logic (provider config, runner, report mapping, menu model, pixel icon). No UI.
- `AIUsage`: the AppKit app (status item, polling, menu).
- `CoreChecks`: the test suite, an executable (`swift run CoreChecks`). XCTest does not build with the Command Line Tools alone.

## Data

- Providers are JSON files (`Resources/providers/*.json`), run as the provider's own CLI. See [data source](data-source/data-source.md).
- Settings (pin, provider switches) are in `UserDefaults`.

## Limits

- Physical memory footprint about 11 MB, must stay under 30 MB.
- Only timer: the 30 s schedule timer. No other polling.
