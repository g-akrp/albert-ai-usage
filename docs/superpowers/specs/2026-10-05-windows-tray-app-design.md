# Windows Tray App Design

**Status:** design approved in chat, spec awaiting review. Nothing implemented.

## Goal

A Windows 11 version of AI Usage that looks and behaves like the macOS menu bar app, on minimum hardware: a single native exe with no runtime to install and a memory footprint under 30 MB.

Windows 11 has no menu bar. The equivalent is the notification area (system tray): the tray icon takes the place of the menu bar icon, and a flyout above it takes the place of the menu panel. Taskbar toolbars (deskbands) were removed in Windows 11, and a floating always-on-top strip over the taskbar is unreliable, so neither is used.

## Decisions

| Topic | Decision |
|---|---|
| Location | `windows/` in this repository, sharing `Resources/providers/*.json` and the specs. |
| Language / UI | Rust, raw Win32 through the `windows` crate, Direct2D and DirectWrite for drawing. No GUI framework. |
| Dependencies | `windows`, `serde_json`. Nothing else at runtime. |
| Platform | Windows 11, x64 and arm64. |
| Install | Per-user MSI (WiX, no admin) and a portable zip, per architecture. |
| Update | Manual: download and run the new MSI. No auto-update. |
| Memory | Under 30 MB, measured as Task Manager "Memory (private working set)". The value is an estimate until measured. |
| macOS app | Unchanged. |

## Repository layout

```
Resources/providers/*.json          shared, unchanged
Resources/providers-windows/*.json  Windows overrides (copilot.json)
docs/specs/windows/                 Windows specs: icon, flyout, runner, release
windows/                            Cargo workspace
  core/                             library, no UI, tested by cargo test
  app/                              binary ai-usage.exe, Win32
  installer/                        WiX source
  scripts/build.ps1
  scripts/release.ps1
```

`AGENTS.md` and `docs/specs/tech-stack.md` are updated to say: the macOS app is Swift and AppKit; the Windows app is Rust and Win32 under `windows/`, written fresh. The old Rust core in git history is not brought back.

## Components

### `windows/core`

A port of `AIUsageCore` with the same behavior. Each module gets its tests first.

- `provider_config`: parses provider JSON in the format of `example/AGENTS.md`, including this repository's additions (`iconLabel`, `iconColor`, `remainingPercent`, `match`, `accounts`).
- `runner`: runs `command` and `stdio` sources, steps, timeouts, and the output limits (`maxOutputBytes`, `maxLineBytes`, `maxTotalBytes`).
- `report`: maps output to meters, windows and percentages.
- `menu_model`, `card_model`: headline percent, value color, ring order, rows, default pin, hidden cards. Same rules as `docs/specs/menubar/`.
- `pixel_icon`: the 3×5 pixel font rendered to an RGBA bitmap. Pure data, no Win32.
- `relative_time`: `in 2 hours`, `resetting…`; takes `now` as an input.
- `exe_lookup`: finds an executable on `PATH` × `PATHEXT`.
- `child_env`: builds the minimal child environment.

### `windows/app`

- `tray`: `Shell_NotifyIconW`, an HICON built from `pixel_icon`, tooltip such as `Claude 10%`. Re-adds the icon when Explorer restarts (`TaskbarCreated` message).
- `flyout`: the card panel window.
- `context_menu`: the native right-click menu.
- `scheduler`: the only timer is a 30 s `SetTimer` on the UI thread. Worker threads run providers, at most two at a time, and send results back with `PostMessage`.
- `settings`: `%APPDATA%\ai-usage\settings.json` holds the pin, hidden cards and providers that are off. A corrupt file is replaced with defaults.
- Single instance through a named mutex; a second launch exits silently.
- A panic writes one line to `%LOCALAPPDATA%\ai-usage\last-error.txt` and exits. Provider output and tokens are never written there.

## Tray icon

- Same two-row pixel label as `docs/specs/menubar/menubar-icon.md`: top row is the provider label (`CLD`, `GH1`, `GEM`) in its brand color, bottom row is the headline percent in green (below 70%), orange (from 70%) or red (from 90%). `ERR` in red for a failed provider, `--` in gray while loading, `AI` over `--` when no provider is on. Headline, color and pin rules are the macOS rules.
- Size follows DPI. At 16 px (100%) the glyphs are drawn at scale 1 and fill 11 × 11 px, centered. At 24 px (150%) scale 2 fills 22 × 22. At other sizes the largest integer scale that fits is used, centered. The icon is redrawn when DPI changes.
- Transparent background. Gray follows the taskbar theme (registry value `SystemUsesLightTheme`).
- Windows 11 puts new tray icons in the `^` overflow and the app cannot change that. Until dismissed, the flyout footer shows a one-time hint: drag the icon onto the taskbar, or turn it on in Settings › Personalization › Taskbar › Other system tray icons.

## Flyout (left click)

- Borderless, top-most popup with no taskbar button (`WS_POPUP`, `WS_EX_TOOLWINDOW`, `WS_EX_TOPMOST`).
- Positioned next to the icon (`Shell_NotifyIconGetRect`) and kept inside the monitor's work area, with the taskbar on any edge.
- Rounded corners through `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE)`. Solid background, no Mica or acrylic. Light or dark follows `AppsUseLightTheme` and switches live.
- Closes on a click outside, `Esc`, or loss of focus. Clicking the icon again toggles it.
- Content follows `docs/specs/menubar/menubar-panel.md`: one card per provider run that is on, each in a rounded outline; title and plan, Copilot account line; concentric rings (at most 4, brand color lightened inward) with the headline percent in the center; one row per ring with a colored dot, label and percent pill; the reset line `Oct 5, 3:20 PM · in 2 hours`; Antigravity group charts; `Loading…`, red error text, `No usage data`, `Usage limit reached`, `All cards are hidden`.
- Hovering a card shows `Hide` and `Pin`; the current pin always shows `Pinned` in the accent color. Pin format is unchanged: `run` or `run|meter`. Clicking a pin or `Hide` closes the flyout.
- Width is the widest card, at least 300 DIP. If the cards are taller than the work area the content scrolls with the mouse wheel.
- Above the footer: configuration errors and `All providers are off`.
- Footer: `Updated 3:21 PM` and a **Refresh** button; `F5` and `Ctrl+R` also refresh while the flyout is open.
- Relative times are computed when the flyout opens and do not tick while it is open.
- Accessibility in v1: the tooltip, and `Tab` / `Enter` to reach and press the Pin and Hide buttons. No full UI Automation tree.

## Context menu (right click)

Refresh Now · Providers ▸ (one check item per provider) · Hidden Cards ▸ (`Show <name>`, only when a card is hidden) · Launch at Login · Open Providers Folder… · `AI Usage <version>` (disabled) · Quit.

Launch at Login writes or removes a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` pointing at the running exe.

## Running providers

### Spawning

- `std::process::Command` with `CREATE_NO_WINDOW`, so no console window flashes. Rust 1.77 and later escape arguments for `.cmd` and `.bat` files safely (the BatBadBut fix).
- `exe_lookup` searches `PATH` with each `PATHEXT` extension and passes the full path. This finds `codex.cmd` from npm and chocolatey shims.
- Each run is assigned to a Job Object with kill-on-job-close. A timeout closes the job, which kills the whole tree (for example an npm shim and its `node` child). This replaces the macOS process group.
- Working folder: a new empty temporary folder, deleted afterwards. Stderr is discarded.

### Environment

- Passed when set: `PATH`, `PATHEXT`, `SystemRoot`, `SystemDrive`, `windir`, `ComSpec`, `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`, `APPDATA`, `LOCALAPPDATA`, `ProgramData`, `TEMP`, `TMP`, `USERNAME`, and `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`, `ALL_PROXY` in both cases.
- Always set: `TERM=dumb`, `NO_COLOR=1`, `HOME` = `USERPROFILE`.
- Then the provider's own `env`.
- `GITHUB_TOKEN` and `GH_TOKEN` from the app's environment are never passed.
- `PATH` is the machine and user `Path` read from the registry at launch and on Refresh Now, so a CLI installed after sign-in is found, followed by `%USERPROFILE%\.local\bin` and `%APPDATA%\npm`.

### Provider files

- Load order: shipped shared files, then shipped Windows overrides, then `%APPDATA%\ai-usage\providers\`. A later file with the same id replaces an earlier one. Shipped files are embedded in the exe with `include_str!`.
- `copilot.json` runs `/bin/sh`, which Windows does not have. `Resources/providers-windows/copilot.json` keeps the same account listing and runs each account with `powershell.exe -NoProfile -NonInteractive -Command`, setting `GH_TOKEN` from `gh auth token --user <account>` inside that child only, then running `gh api copilot_internal/user`. The token is never stored or logged.
- **Unconfirmed:** how `${account}` reaches the PowerShell 5.1 script as an argument. The first Copilot task in the plan checks this by hand before anything depends on it.

### Scheduling

Same as macOS: 5 minutes per provider (Antigravity 10), at most two at a time, failure backoff of 1, 2, 4, … minutes up to 30. Refresh Now runs every provider and rereads provider files. A file that does not load is listed with the reason.

## Build, install, release

- **Prerequisites (developer machine, installed by the human):** Visual Studio Build Tools 2022 with "Desktop development with C++", the MSVC ARM64 build tools and the Windows 11 SDK; `rustup default stable`; `rustup target add aarch64-pc-windows-msvc`; the WiX v5 .NET tool. At the time of writing this machine has `rustup` with no toolchain installed and no MSVC or Windows SDK found.
- **Build:** `windows/scripts/build.ps1 [-Arch x64|arm64|all]` builds `ai-usage.exe` for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` with a static CRT, an embedded icon and a version resource.
- **Installer:** per-user MSI built with WiX v5 (build time only, not shipped). Installs to `%LOCALAPPDATA%\Programs\AI Usage\` without admin rights, adds a Start menu shortcut, and appears in Settings › Apps for uninstall. Uninstall removes the Run value and keeps `%APPDATA%\ai-usage\`. A fixed UpgradeCode with `MajorUpgrade` lets a newer MSI replace an older one. **Unconfirmed:** the exact WiX v5 per-user attributes, and whether the installer closes a running app or asks the user to quit it; both are checked against the WiX documentation in the plan.
- **Portable:** a zip per architecture containing `ai-usage.exe`.
- **Release:** `windows/scripts/release.ps1 <version> [-Publish]` sets the version in `windows/app/Cargo.toml`, runs `cargo test`, builds both architectures, and writes `AIUsage-win-<version>-x64.msi`, `-arm64.msi`, `-x64.zip`, `-arm64.zip` and `SHA256SUMS`, then commits and tags `win-v<version>`. `-Publish` pushes and creates the GitHub release. Windows and macOS versions are independent. Nothing is pushed or published unless the human asks.
- **Unsigned:** SmartScreen warns on first run (More info › Run anyway). The README says so.

## Testing

- `cargo test -p core`, written before each module: config parsing, report mapping from `example/*.json` and fixtures, headline percent and color thresholds, ring order, default pin, hidden cards, relative time, pixel icon bitmaps, `PATH` × `PATHEXT` lookup, environment filter (never `GH_TOKEN`).
- Runner tests spawn small `.cmd` fixture scripts to check timeouts, killing the whole tree, and output limits.
- `ai-usage.exe --live` runs the shipped providers and prints what the flyout would show, like `CoreChecks --live`.
- Manual checklist: DPI 100, 125, 150 and 200%; light and dark; taskbar restart through Explorer; two monitors; memory under 30 MB in Task Manager; install, upgrade and uninstall with the MSI; arm64 on a real device if one is available, otherwise marked unconfirmed.

## Out of scope for v1

Auto-update, MSIX, code signing, Mica or acrylic, a full UI Automation tree.
