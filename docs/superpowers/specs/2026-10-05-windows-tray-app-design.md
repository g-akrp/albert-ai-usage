# Windows Tray App Design

**Status:** approved planning snapshot, updated 2026-10-10. Nothing implemented. Follow [the current implementation plan](../../changes/windows-tray-app.md) for task order, current UI parity, WiX 7, and verification gates.

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
| Install | Per-user MSI (WiX 7, no admin) and a portable zip, per architecture. |
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

Before coding, update `AGENTS.md` and add `windows/AGENTS.md` for the approved scope: macOS remains Swift/AppKit; Windows is fresh Rust/Win32 under `windows/`. Sync `docs/specs/tech-stack.md` to verified behavior after implementation. The old Rust core in Git history is not brought back.

## Components

### `windows/core`

A port of `AIUsageCore` with the same behavior. Each module gets its tests first.

- `provider_config`: parses provider JSON in the format of `example/AGENTS.md`, including this repository's additions (`iconLabel`, `iconColor`, `remainingPercent`, `match`, `accounts`).
- `runner`: runs `command` and `stdio` sources, steps, timeouts, and the output limits (`maxOutputBytes`, `maxLineBytes`, `maxTotalBytes`).
- `report`: maps output to meters, windows and percentages.
- `menu_model`, `card_model`, `card_order`: headline percent, value color, ring order, counted pills, rows, default pin, hidden/collapsed cards, and saved ordering. Same rules as the current `docs/specs/menubar/`.
- `pixel_icon`: the 3×5 pixel font rendered to an RGBA bitmap. Pure data, no Win32.
- `relative_time`: compact durations such as `in 3h 25m`, `in 4d 2h 12m`, and `resetting…`; takes `now` as an input.
- `exe_lookup`: finds an executable on `PATH` × `PATHEXT`.
- `child_env`: builds the minimal child environment.

### `windows/app`

- `tray`: `Shell_NotifyIconW`, an HICON built from `pixel_icon`, tooltip such as `Claude 10%`. Re-adds the icon when Explorer restarts (`TaskbarCreated` message).
- `flyout`: the card panel window.
- `context_menu`: the native right-click menu.
- `scheduler`: the only timer is a 30 s `SetTimer` on the UI thread. Worker threads run providers, at most two at a time, and send results back with `PostMessage`.
- `settings`: `%APPDATA%\ai-usage\settings.json` holds the pin, hidden/collapsed cards, saved card order, provider switches, and dismissed tray hint. A corrupt file uses defaults; saving replaces it atomically.
- Single instance through a named mutex; a second launch exits silently.
- A panic writes one line to `%LOCALAPPDATA%\ai-usage\last-error.txt` and exits. Provider output and tokens are never written there.

## Tray icon

- Same two-row pixel label as `docs/specs/menubar/menubar-icon.md`: top row is the provider label (`CLD`, `GH1`, `GEM`) in its brand color, bottom row is the headline percent in green (below 70%), orange (from 70%) or red (from 90%). `ERR` in red for a failed provider, `--` in gray while loading, `AI` over `--` when no provider is on. Headline, color and pin rules are the macOS rules.
- Size follows DPI. Measure the actual longest label/value, including `100%`, and choose the largest integer glyph scale that fits the target icon dimensions. Center the result and redraw on DPI changes; do not assume every glyph string fits 11 × 11 px.
- Transparent background. Gray follows the taskbar theme (registry value `SystemUsesLightTheme`).
- Windows 11 puts new tray icons in the `^` overflow and the app cannot change that. Until dismissed, the flyout footer shows a one-time hint: drag the icon onto the taskbar, or turn it on in Settings › Personalization › Taskbar › Other system tray icons.

## Flyout (left click)

- Borderless, top-most popup with no taskbar button (`WS_POPUP`, `WS_EX_TOOLWINDOW`, `WS_EX_TOPMOST`).
- Positioned next to the icon (`Shell_NotifyIconGetRect`) and kept inside the monitor's work area, with the taskbar on any edge.
- Rounded corners through `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE)`. Solid background, no Mica or acrylic. Light or dark follows `AppsUseLightTheme` and switches live.
- Closes on a click outside, `Esc`, or loss of focus. Clicking the icon again toggles it.
- Content follows the current `docs/specs/menubar/menubar-panel.md`: one outlined card per enabled provider run; title/plan and account line; concentric rings and per-window rows; colored usage pills; compact local reset labels; grouped Antigravity charts; and loading/error/access/unavailable states. Copilot Premium Interactions includes used/limit counts and any reported credits unit, with resets mapped from `/quota_reset_date_utc`.
- Default order is Claude Code, Codex, Antigravity, GitHub Copilot. Hover/focus shows `↑`, `↓`, `Hide`, and `Pin`; the current pin always shows `Pinned`. Move arrows swap nearest visible cards without moving hidden cards. A title-row chevron collapses to percentage-only summary pills while keeping account lines and errors/notices visible. Persist order and collapse by run id. Pin format remains `run` or `run|meter`; pin, Hide, move, and collapse actions close the flyout.
- Width is the widest card, at least 300 DIP. If the cards are taller than the work area the content scrolls with the mouse wheel.
- Above the footer: configuration errors and `All providers are off`.
- Footer follows macOS menu order: Refresh Now, Updated time, Providers, optional Hidden Cards, Launch at Login, Open Providers Folder, version, author credit, and Quit. `F5` and `Ctrl+R` refresh while the flyout is open.
- Relative times are computed when the flyout opens and do not tick while it is open.
- Accessibility in v1: tooltip/native menus and `Tab` / `Enter` / `Space` for card/menu actions, with `Esc` to close. No full custom UI Automation tree.

## Context menu (right click)

Refresh Now · Providers ▸ (one check item per provider) · Hidden Cards ▸ (`Show <name>`, only when a card is hidden) · Launch at Login · Open Providers Folder… · `AI Usage <version>` (disabled) · author credit · Quit.

Launch at Login writes or removes a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` pointing at the running exe.

## Running providers

### Spawning

- Use a tested Windows process adapter with `CREATE_NO_WINDOW`. Create the initial process suspended, assign its Job Object, then resume it. Dispatch native executables directly; test a separate `.cmd`/`.bat` adapter for argument escaping and paths with spaces.
- `exe_lookup` searches `PATH` with each `PATHEXT` extension and passes the full path. This finds `codex.cmd` from npm and chocolatey shims.
- Each capture owns a Job Object with kill-on-job-close. Assign before resume to avoid child escape between spawn and assignment. A timeout/quit kills the owned process tree, including shim descendants; assignment failure cleans up the suspended process.
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
- Pass `${account}` in a dedicated environment variable to a static PowerShell 5.1 command. Verify with synthetic `gh` fixtures before live usage. Preserve the latest shared account/count/unit/reset mappings and do not add `-ExecutionPolicy Bypass` for the inline adapter. Actual logged-in behavior remains unverified.

### Scheduling

Same as macOS: 5 minutes per provider (Antigravity 10), at most two at a time, failure backoff of 1, 2, 4, … minutes up to 30. Refresh Now runs every provider and rereads provider files. A file that does not load is listed with the reason.

## Build, install, release

- **Prerequisites, checked 2026-10-10:** Rust/Cargo 1.99.0, x64/ARM64 MSVC targets, Visual Studio Build Tools 2022/MSVC 14.44, Windows SDKs 10.0.22621.0 and 10.0.26100.0, .NET SDK 8.0.425, and WiX 7.0.0 are present. x64/ARM64 native smoke builds passed; x64 ran successfully. MSI commands currently fail with WIX7015 because WiX EULA acceptance is pending. The existing `.gitignore` merge conflict also remains unresolved.
- **Build:** `windows/scripts/build.ps1 [-Arch x64|arm64|all]` builds `ai-usage.exe` for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` with a static CRT, an embedded icon and a version resource.
- **Installer:** WiX 7, pinned by a local .NET tool manifest and used only at build time. `Scope="perUser"` supports installation without elevation; see [WiX package scope](https://docs.firegiant.com/wix/schema/wxs/packagescopetype/). Install to `%LOCALAPPDATA%\Programs\AI Usage\`, add a Start menu shortcut/uninstall entry, use a stable upgrade identity, remove the app's Run value on uninstall, and retain `%APPDATA%\ai-usage\`. Verify running-app handling, upgrade/repair/uninstall, and architecture behavior before claiming them supported. The human/build owner must review and accept [WiX 7 terms](https://docs.firegiant.com/wix/osmf/); scripts must not accept them silently.
- **Portable:** a zip per architecture containing `ai-usage.exe`.
- **Release:** `windows/scripts/release.ps1 <version>` sets Windows version fields through the script, runs checks/builds, and creates per-architecture MSI/ZIP artifacts and SHA-256 checksums. Default behavior is local preparation only: no commit, tag, push, publish, installer execution, or EULA acceptance. Git/release actions require a separate explicit human request. Windows and macOS versions are independent.
- **Unsigned:** SmartScreen warns on first run (More info › Run anyway). The README says so.

## Testing

- `cargo test --manifest-path windows/Cargo.toml -p ai-usage-core`, written before each core module: configuration/report mapping, counts/units/resets, headline/tone boundaries, ring/group order, pinning, collapse, hidden cards, saved ordering, compact time, pixels, lookup, environment filtering, and scheduler decisions.
- Runner tests spawn small `.cmd` fixture scripts to check timeouts, killing the whole tree, and output limits.
- `ai-usage.exe --live` runs the shipped providers and prints what the flyout would show, like `CoreChecks --live`.
- Manual checklist: DPI 100, 125, 150 and 200%; light and dark; taskbar restart through Explorer; two monitors; memory under 30 MB in Task Manager; install, upgrade and uninstall with the MSI; arm64 on a real device if one is available, otherwise marked unconfirmed.

## Out of scope for v1

Auto-update, MSIX, code signing, Mica or acrylic, a full UI Automation tree.
