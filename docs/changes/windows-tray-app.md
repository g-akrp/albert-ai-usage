# Native Windows AI Usage implementation plan

Status: implementation in progress
Date: 2026-10-10
Platform: Windows 11, x64 and ARM64
Installer: WiX 7

## Problem and approved scope

Windows users need the same usage monitoring that the current macOS app provides. The approved design replaces the macOS menu bar item with a Windows notification-area icon and preserves the provider cards, pinning, collapse, ordering, counts, reset times, and provider controls.

This plan is the current implementation entry point. It supersedes the task sequence in [the earlier Windows plan](../superpowers/plans/2026-10-05-windows-tray-app.md). [The Windows design](../superpowers/specs/2026-10-05-windows-tray-app-design.md) supplies additional platform details. The current Swift sources and [macOS panel specification](../specs/menubar/menubar-panel.md) are the behavior reference; the earlier mockup's sample values are not fixtures or live usage.

The user approved the design, WiX 7 selection, and implementation in chat. This does not accept the WiX EULA or authorize committing/publishing.

## Implementation spec

### Architecture

Build a fresh Cargo workspace under `windows/`. Keep the macOS application in Swift/AppKit and leave its runtime architecture intact. Do not restore the old Rust core from Git history or create a shared C ABI.

| Layer | Responsibility | Dependencies |
| --- | --- | --- |
| `ai-usage-core` | Provider configuration, JSON mapping, usage/card models, settings models, ordering, scheduling decisions, icon pixels, executable lookup, and bounded provider execution | `serde_json`; `windows` for Windows process operations |
| `ai-usage` | Win32 event loop, notification-area icon, Direct2D/DirectWrite flyout, native menus, registry integration, settings persistence, worker coordination | `windows`, local `ai-usage-core` |
| Build and packaging | Embedded manifest/icon/version resources, static CRT, release artifacts, per-user MSI | Cargo/MSVC, `embed-resource` at build time, WiX 7 at build time |

No web view, Electron, Tauri, Flutter, SwiftUI, GUI framework, helper daemon, async runtime, or third-party date/time library. The HTML design shown in chat is a review aid and is not shipped.

```text
Resources/providers/*.json             shared provider definitions
Resources/providers-windows/copilot.json
windows/
  Cargo.toml
  Cargo.lock
  .cargo/config.toml
  .config/dotnet-tools.json             pinned WiX 7 local tool
  AGENTS.md
  core/
    Cargo.toml
    src/
    tests/
    tests/fixtures/
  app/
    Cargo.toml
    build.rs
    src/
    resources/
  installer/Package.wxs
  scripts/check-prereqs.ps1
  scripts/build.ps1
  scripts/package.ps1
  scripts/release.ps1
  scripts/check-memory.ps1
  dist/                                generated, ignored
docs/specs/windows/                    verified behavior after implementation
```

### Contracts and data flow

1. `ProviderStore` merges embedded shared definitions, Windows overrides, and user files, in that order. Later files with the same provider id replace earlier files. Invalid files become configuration errors without stopping other providers.
2. `Scheduler` selects due provider runs. Workers return `ProviderRun` results through a bounded queue and a posted Win32 message; only the UI thread mutates displayed state.
3. `Report` retains windows, used percentages, optional counts (`used`, `limit`, `unit`), reset timestamps, plan/account labels, and unavailable/access/error states.
4. `CardModel` derives ring order, headline percentage, pills, pin targets, collapsed summaries, and visible notices. `PanelModel` combines cards, configuration errors, and menu items.
5. `Settings` persists `pinnedProvider`, `hiddenCards`, `disabledProviders`, `collapsedCards`, `cardOrder`, and `overflowHintDismissed`. Run ids distinguish Copilot accounts. A card hidden from the panel continues to refresh; a disabled provider does not run.
6. `PanelLayout` contains measured rectangles and explicit hit targets for collapse, title pin, group/limit pin, Hide, move arrows, and menu actions. Rendering and hit testing consume the same layout.
7. The tray image is derived from the selected provider or limit. Collapse and ordering do not alter a still-valid pin. An invalid/disabled pin falls back to the first enabled provider in panel order, with Antigravity selecting its first group.

### UI parity with the current macOS app

- Default order: Claude Code, Codex, Antigravity, GitHub Copilot. Multiple Copilot accounts retain their discovery order until explicitly moved.
- One outlined card per enabled provider run; provider name and plan in parentheses, plus a separate Copilot account line.
- Concentric rings: session/five-hour window first, then weekly, then remaining windows. Use at most four rings; do not discard additional reported rows.
- Headline priority: session, weekly, premium interactions, then highest reported usage. Tray value color uses the highest usage of the pinned target, with warning thresholds at 70% and 90%.
- Antigravity gets one chart per model group. Its title pin selects the first group; group pins select only that group. Claude and Codex pin the provider; Copilot can pin its provider or one limit.
- Expanded rows have a ring-colored dot, label, usage-colored pill, reset date/time in the user's timezone, and compact time left such as `in 3h 25m` or `in 4d 2h 12m`.
- Copilot Premium Interactions retains counts: for example `18% · 54/300`, or an appended `credits` unit when the provider reports credit-based billing. Map the actual `/quota_reset_date_utc` field; do not invent a reset date. Skip unlimited Chat/Completions as the shared provider definition does.
- A chevron collapses a card to its title and percentage-only pills. Keep the account line and errors/notices visible. Persist collapse by run id. Clicking the title pins; it does not collapse.
- Hover/focus exposes `↑`, `↓`, Hide, and Pin. The current Pinned button remains visible. Moving swaps nearest visible neighbors, skips hidden/disabled cards, and preserves hidden-card positions. No drag-and-drop reordering.
- Pin, Hide, collapse, and move actions close the flyout. Reopen from the tray icon. Keyboard users can reach actions with Tab and activate them with Enter/Space; Escape closes the flyout.
- After the cards: configuration errors, all-off/all-hidden states, Refresh Now, Updated time, Providers, Hidden Cards when needed, Launch at Login, Open Providers Folder, version, author credit, and Quit. Use Ctrl+R and F5 for refresh.
- Use an opaque Windows-themed surface, Segoe UI typography, rounded corners, and no Mica/acrylic. Follow app appearance changes without introducing a polling timer.
- Anchor to the tray icon, including overflow, with a monitor-work-area fallback. Handle negative monitor coordinates, DPI changes, and taskbar edges. Scroll only when the actual flyout exceeds the monitor work area.
- Re-add the icon after Explorer restarts. Keep the application out of the taskbar and enforce a single instance. Offer the existing one-time tray-visibility hint without trying to change Windows tray settings automatically.

### Execution, credentials, and resource bounds

- Provider changes remain JSON changes. Embed shipped definitions and load user overrides from `%APPDATA%\ai-usage\providers\`.
- Resolve executable paths using Windows PATH/PATHEXT semantics and common CLI install directories. Dispatch native executables directly. Route `.cmd`/`.bat` shims through a specifically tested command-line adapter; never concatenate untrusted values into shell script text.
- Use a fresh empty working directory per capture, hidden process windows, bounded stdout, and discarded stderr. Handle UTF-8 BOM and CRLF. Preserve command and newline-delimited stdio protocols and their whole-run deadlines.
- Put each capture in an owned Job Object with kill-on-close. Create the initial process suspended, assign it to the job, then resume it so children cannot escape before assignment. On assignment failure, terminate the owned suspended process and report the failure. This ordering is an implementation decision to eliminate the spawn/assignment race; [Microsoft documents job assignment and inherited job membership](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject).
- Whitelist required environment variables, rebuild PATH on manual refresh, and strip inherited `GITHUB_TOKEN`/`GH_TOKEN`. Do not read, store, or log credentials, raw authentication output, or provider stdout in diagnostics.
- The Windows Copilot JSON override uses a static PowerShell command. Pass the account in a dedicated environment variable. Only that CLI-owned child obtains its login token and calls `gh api`; the app never captures the token. Preserve the current shared mapping for counts, units, resets, account discovery, and quota filtering. Do not add `-ExecutionPolicy Bypass` for an inline `-Command` adapter.
- One 30-second schedule timer, at most two concurrent captures, default five-minute intervals and Antigravity's ten-minute interval. Use the existing failure backoff up to 30 minutes. Event-driven completion and bounded waits replace busy polling. Prevent duplicate overlapping runs during repeated manual refresh.
- Reset labels are computed when the panel opens or refreshes; they do not need another timer while the panel is open.
- Target less than 30 MiB of application private working set in release builds. Record provider child memory separately; do not hide child costs or substitute private committed bytes for private working set.
- Shut down workers/jobs, handles, tray icons, drawing resources, and temporary folders predictably. Persist settings with an atomic replacement strategy. Malformed settings use defaults without crashing.

### WiX 7 packaging decision and acceptance gate

Use WiX 7.0.0 initially, pinned through a local .NET tool manifest created by `dotnet` tooling. Keep the installed global tool out of version selection for reproducible packaging. WiX stays a build dependency; users do not install WiX or .NET to run the native app.

Use `Package Scope="perUser"`, install under `%LOCALAPPDATA%\Programs\AI Usage\`, and add a per-user Start menu shortcut and uninstall entry. [WiX documents that `perUser` does not require elevation](https://docs.firegiant.com/wix/schema/wxs/packagescopetype/). Add a stable UpgradeCode, architecture-specific package output, and tested major-upgrade behavior. Uninstall removes application files and the app's Launch at Login registration, while preserving `%APPDATA%\ai-usage\` user settings/providers.

WiX 7 enforces explicit EULA acceptance. The current machine returned WIX7015 from `wix build --help`, so MSI packaging is blocked until the human reviews the [OSMF/EULA terms](https://docs.firegiant.com/wix/osmf/) and completes acceptance. Native compilation and portable ZIP work can proceed independently.

- Do not execute EULA acceptance as part of this planning task.
- Do not put `-acceptEula wix7` or `AcceptEula` into scripts/CI without explicit authorization to accept those terms in that environment.
- `check-prereqs.ps1` distinguishes native-build readiness from MSI readiness and reports the acceptance failure clearly. Build/package scripts fail rather than accept silently.
- Confirm the running-app upgrade/uninstall path using Windows Installer's normal files-in-use/Restart Manager behavior. Prefer graceful shutdown; do not force-terminate the app or request elevation to avoid a prompt. If a WiX extension is required, add it only as a pinned build dependency after reproducing the need. [CloseApplication options](https://docs.firegiant.com/wix/schema/util/closeapplication/) require a separate implementation/test decision.
- MSI install/upgrade/uninstall tests and ARM64 runtime tests remain unverified until performed on suitable machines. Cross-architecture upgrade behavior must be documented from a real test before claiming support.

## Prerequisite audit, 2026-10-10

| Check | Observed result |
| --- | --- |
| Rust/Cargo | 1.99.0; stable MSVC toolchain |
| Rust targets | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` |
| Visual Studio | Build Tools 2022, complete; MSVC 14.44.35207 x64 and ARM64 tools |
| Windows SDK | 10.0.22621.0 and 10.0.26100.0; Direct2D/DirectWrite headers and target libraries present |
| Native smoke test | x64 and ARM64 compile/link passed; x64 binary ran successfully |
| .NET/WiX | .NET SDK 8.0.425; WiX 7.0.0 installed; EULA acceptance pending |
| Provider commands | `claude`, `codex`, `gh`, `agy` found; login/usage not inspected by the audit |
| Repository | Existing unresolved `.gitignore` merge conflict; user changes already staged |

Toolchain presence and smoke tests do not establish that the future application meets memory, UI, or installer acceptance criteria.

## Plan

Dependencies run in order unless a task explicitly says otherwise. For core behavior, write the check first, observe the expected failure, implement, and rerun the relevant checks. Do not mark a task complete from code inspection alone. Use synthetic provider fixtures for automated tests; live account checks are separate.

### 1. Prepare the workspace and scope

Files: `AGENTS.md`, `windows/AGENTS.md`, `windows/Cargo.toml`, `windows/core/Cargo.toml`, `windows/app/Cargo.toml`, `windows/.cargo/config.toml`, `.gitignore`.

- [x] Inspect and resolve the existing `.gitignore` conflict without discarding staged/user changes. Record the initial Git status; never commit the merge implicitly.
- [x] Update repository instructions for the approved Windows directory, preserving all macOS constraints and the ban on resurrecting historical Rust code.
- [ ] Scaffold the two crates, minimal dependencies, and static-CRT targets. Select compatible crate versions, commit the lockfile only when asked, and enable only required Win32 features.
- [ ] Confirm rustfmt/clippy components and create `windows/scripts/check-prereqs.ps1` with separate native and MSI results.
- [ ] Verify native builds for both targets. No WiX acceptance is needed at this stage.

Verification: `git diff --name-only --diff-filter=U` is empty; `cargo test --manifest-path windows/Cargo.toml --workspace`; `cargo check --manifest-path windows/Cargo.toml --workspace --target aarch64-pc-windows-msvc`.

### 2. Port provider configuration and report mapping

Files: `windows/core/src/config.rs`, `json.rs`, `report.rs`; `windows/core/tests/config.rs`, `report.rs`, `fixtures/`.

- [ ] Port relevant `Sources/CoreChecks/ModelChecks.swift` cases first, including validation limits, JSON pointers, accounts, revision handling, matching, remaining percentages, and BOM/CRLF input.
- [ ] Add fixtures for session/weekly ordering, multiple groups/accounts, unavailable/access flags, and malformed or incomplete data.
- [ ] Cover Copilot `entitlement - quota_remaining`, credit units, zero/exhausted quota, unlimited filtering, and root UTC reset dates. Preserve counts in the report rather than rebuilding them from rounded percentages.
- [ ] Implement the current format without changing shared JSON or Swift behavior. Keep parse errors bounded and free of raw provider output.

Verification: `cargo test --manifest-path windows/Cargo.toml -p ai-usage-core --test config --test report`.

### 3. Port card models, pinning, collapse, order, time, and pixels

Files: `windows/core/src/card_model.rs`, `menu_model.rs`, `card_order.rs`, `relative_time.rs`, `pixel_icon.rs`; corresponding integration tests.

- [ ] Check headline priority and highest-usage tone independently, including boundary values 69/70/89/90/100.
- [ ] Check flat Copilot charts versus Antigravity group charts, per-limit/group pins, default fallback, and account labels.
- [ ] Check percentage-only collapsed pills; group headline values with worst-ring tone; visible errors/notices in collapsed cards.
- [ ] Check default order, saved order, unknown/new ids, nearest-visible moves, and hidden-card positions. Hide must not disable execution or invalidate a valid pin.
- [ ] Port compact reset formatting with deterministic `now`, including boundary cases and `resetting…`; inject the local-date formatter from the UI adapter.
- [ ] Check all pixel glyphs and bitmap sizes using the longest label/value. Choose integer scaling from actual glyph width/height, including `100%`, rather than assuming every label fits an 11-pixel width.

Verification: `cargo test --manifest-path windows/Cargo.toml -p ai-usage-core --test models --test time --test pixels`.

### 4. Implement bounded Windows subprocess execution

Files: `windows/core/src/exe_lookup.rs`, `child_env.rs`, `process.rs`, `runner.rs`; `windows/core/tests/runner.rs`; native and shim fixture programs/scripts.

- [ ] Check full paths and folders with spaces, PATH/PATHEXT lookup, `.cmd` arguments with spaces/quotes/metacharacters, and account values passed as environment data.
- [ ] Check command and stdio captures, out-of-order/non-JSON lines, CRLF, BOM, stdin EOF, nonzero exit, timeout, per-line/total/output caps, and early child failure.
- [ ] Use dummy token sentinels to verify inherited token variables are removed without touching real credentials.
- [ ] Check job assignment before resume, failed assignment cleanup, and timeout/quit cleanup of exact owned child/grandchild PIDs. Do not count or terminate unrelated system processes.
- [ ] Implement RAII handles and per-run temporary directories; enforce limits during reading, not only after capture. Reap processes and close reader threads before releasing results.

Verification: `cargo test --manifest-path windows/Cargo.toml -p ai-usage-core --test runner`. Capture evidence that fixture descendants exit and owned handles/temp directories return to baseline.

### 5. Add provider store and Windows Copilot adapter

Files: `windows/core/src/store.rs`; `Resources/providers-windows/copilot.json`; `windows/core/tests/store.rs`, `copilot_adapter.rs`.

- [ ] Check embedded/shared/Windows/user precedence, matching file ids, size/type constraints, unreadable files, and nonfatal configuration errors.
- [ ] Start from the latest shared Copilot definition. Replace only its platform-specific command adapter; retain account and report mappings. Generate provider revision through the repository's established provider-edit convention.
- [ ] Verify the static PowerShell 5.1 adapter with a fake `gh` that returns synthetic account/token/quota data. Its only app-visible stdout must be the final usage JSON; auth failures must not emit tokens.
- [ ] Add an explicit opt-in live usage smoke mode that prints a sanitized usage summary. Do not print source stdout, child environment, or login tokens.

Verification: `cargo test --manifest-path windows/Cargo.toml -p ai-usage-core --test store --test copilot_adapter`. Actual logged-in provider behavior is a later manual check.

### 6. Persist settings and implement scheduling decisions

Files: `windows/core/src/settings_model.rs`, `schedule.rs`; `windows/app/src/settings.rs`, `scheduler.rs`; settings/schedule tests.

- [ ] Check malformed/truncated/unknown settings, atomic replacement, new/removed account reconciliation, saved collapse/order, valid pin preservation, and user-folder handling.
- [ ] Check due intervals and failure backoff with an injected clock, two-run concurrency, duplicate suppression, disabled providers, manual-refresh prioritization, and shutdown cancellation.
- [ ] Wire one 30-second timer and completion messages. Refresh Now reloads provider files and PATH and reruns enabled providers; hidden cards remain eligible.

Verification: `cargo test --manifest-path windows/Cargo.toml -p ai-usage-core --test settings --test schedule`; inspect the application timer creation sites and verify no polling loops were added.

### 7. Build the native tray shell and menus

Files: `windows/app/src/main.rs`, `tray.rs`, `context_menu.rs`, `login.rs`; app resource manifest and icon.

- [ ] Add the single-instance mutex, Win32 message loop, tray registration/callbacks, DPI-aware HICON generation, and Explorer-restart handling.
- [ ] Add native menu actions and matching flyout actions, Launch at Login registration with a quoted executable path, and Open Providers Folder.
- [ ] Handle quit/end-session by saving settings, cancelling owned work, removing the icon, and releasing resources. Diagnostics contain only safe operation/error labels.
- [ ] Verify first launch, second launch, no taskbar button, missing CLI/error states, zero enabled providers, tray overflow, and Explorer restart.

Verification: launch the x64 release executable; exercise each action. Treat provider login as the CLI's responsibility; the app does not prompt for credentials.

### 8. Implement layout, drawing, and interaction parity

Files: `windows/core/src/panel_layout.rs`; `windows/core/tests/panel_layout.rs`; `windows/app/src/flyout.rs`, `draw.rs`, `appearance.rs`, `time_format.rs`.

- [ ] Check measured layout and hit targets for long plans/accounts, count/credit pills, four rings, grouped cards, collapsed cards, hover controls, and error/configuration rows.
- [ ] Check DPI scaling, monitor edges/negative coordinates, absent/overflow tray rectangles, and work-area clamping. Use the measured layout for pointer and keyboard actions.
- [ ] Draw the current card/menu design with Direct2D and DirectWrite. Recover drawing resources after device loss; respond to theme/DPI messages.
- [ ] Add scroll handling when required, Tab/Enter/Space/Escape, Ctrl+R/F5, focus/outside-click dismissal, and close-on-card-action behavior.
- [ ] Complete the UI parity checklist, including author credit, collapsed error visibility, order arrows, and Copilot counts/resets. Basic tooltip/menu/keyboard accessibility is in scope; a complete custom UI Automation tree is deferred and must not be claimed.

Verification: `cargo test --manifest-path windows/Cargo.toml -p ai-usage-core --test panel_layout`; manual checks at 100/125/150/200% DPI, light/dark appearance, multiple monitors, and tray overflow.

### 9. Build portable artifacts and measure resources

Files: `windows/app/build.rs`, `windows/app/resources/`; `windows/scripts/build.ps1`, `check-memory.ps1`; packaging helper tests.

- [ ] Build release executables for x64 and ARM64 with embedded icon, manifest, generated version resources, and static CRT. Versions are set by the release script, not edited by hand.
- [ ] Create one portable ZIP per architecture and test extraction/startup from a path with spaces.
- [ ] Measure release private working set using Task Manager or `Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate`, selected by the exact app PID. Record idle/open/refresh peaks, handles, and provider-child memory separately.
- [ ] Run for at least 15 minutes, open/close the flyout repeatedly, refresh, and exercise failures. Require application private working set below 30 MiB with no sustained handle/memory growth. Investigate a failed target before marking this task complete.
- [ ] Run the ARM64 build on actual Windows ARM64 hardware when available. A successful cross-build is not a runtime pass; record hardware execution as pending when unavailable.

Verification: `powershell -NoProfile -File windows/scripts/build.ps1 -Arch all`; `powershell -NoProfile -File windows/scripts/check-memory.ps1`; portable smoke checks on x64 and, separately, ARM64.

### 10. Implement WiX 7 per-user MSI packaging

Files: `windows/.config/dotnet-tools.json`; `windows/installer/Package.wxs`; `windows/scripts/package.ps1`; installer validation records.

- [ ] Create the local tool manifest with `dotnet` commands and pin WiX 7.0.0. Check restored tool version. Keep EULA acceptance as the separate human/CI-owner action described above.
- [ ] Add per-user package scope, the LocalAppData installation path, Start menu shortcut, stable upgrade identity, architecture/version inputs, uninstall registration, and cleanup of the app's Run value.
- [ ] Invoke the local tool from `windows/` using `dotnet tool run wix -- build ...`, with x64/ARM64 architecture and generated version/executable paths passed as arguments. Do not add silent EULA acceptance switches.
- [ ] Validate MSI tables/scope and test installation using a standard non-admin user. Require no elevation prompt and no runtime/.NET installation.
- [ ] Test upgrade over an earlier test package, running-app files-in-use behavior, downgrade rejection, repair, cancellation, and uninstall. Preserve settings/user providers; remove installed files, shortcut, uninstall entry, and Launch at Login value as intended.
- [ ] Repeat architecture-specific packaging checks. Document cross-architecture upgrade results and any ARM64 hardware gap without claiming untested behavior.

Verification: `powershell -NoProfile -File windows/scripts/package.ps1 -Arch all -Version <release-script-generated-version>` after EULA acceptance; install/upgrade/uninstall logs and MSI table inspection. These checks cannot pass while WIX7015 remains unresolved.

### 11. Finish release automation and shipped documentation

Files: `windows/scripts/release.ps1`; `README.md`; `docs/specs/windows/{icon,panel,runner,release}.md`; `docs/specs/tech-stack.md`.

- [ ] Release preparation validates version input, sets Windows version fields/resources through the script, runs checks, builds both architectures, packages MSIs/ZIPs, and generates SHA-256 checksums. Windows versioning remains independent of macOS versioning.
- [ ] Default script behavior creates local artifacts only. It must not commit, tag, push, publish, install an MSI, or accept terms. Those actions require a separate explicit human request.
- [ ] Add CI only when requested or needed for implementation verification; any MSI job uses the same pinned tool and explicitly approved acceptance setup. Keep signing and secrets outside this plan's scope.
- [ ] Document MSI/portable installation, CLI prerequisites/login ownership, tray visibility, manual updates, launch-at-login, supported architectures, and current signing status.
- [ ] Sync specs to verified shipped behavior rather than presenting unfinished design as implemented. Preserve macOS-only architecture statements where appropriate and add a clearly scoped Windows section.
- [ ] Run the macOS CoreChecks on a Mac if shared files or existing macOS behavior changed. Do not claim macOS checks passed from this Windows host.

Verification: `cargo fmt --manifest-path windows/Cargo.toml --all -- --check`; `cargo clippy --manifest-path windows/Cargo.toml --workspace --all-targets -- -D warnings`; `cargo test --manifest-path windows/Cargo.toml --workspace`; both release targets; complete manual/installer/memory records.

## Completion criteria and remaining gates

### Portable-only scope update

The user chose "Keep portable ZIPs only for now". MSI acceptance, build, and installer validation are deferred. WiX terms remain unaccepted. Native implementation and portable build scripts are present; 25 core/fixture checks pass. x64 debug UI and Refresh were observed. Computer Use was stopped by the user, so remaining UI checks are pending. Release memory endurance and ARM64 runtime are also pending. This is a preview, not completion of every validation gate below. Current distribution commands omit a `-Version` parameter: `package.ps1 -Arch all -PortableOnly` reads the workspace version.

- [ ] Windows core checks pass and runner fixtures leave no owned subprocesses behind.
- [ ] Native tray/flyout behavior matches the current macOS contracts, including collapse, order, counts, account handling, and reset formatting.
- [ ] No credentials are inherited, captured by the app, persisted, or included in logs.
- [ ] One schedule timer, bounded concurrency/output, stable handle count, and release private working set below 30 MiB.
- [ ] x64 release executable and portable ZIP run successfully; ARM64 build and runtime evidence are reported separately.
- [ ] WiX 7 terms accepted by the human/build owner; per-user MSI install/upgrade/uninstall behavior verified without elevation.
- [ ] User work preserved; merge conflict resolved before coding integration; no unsolicited commit/push/publish.
- [ ] Windows specs and README describe verified behavior. Keep Status as implementation not started until implementation actually begins, and mark implemented only after required work is complete.

Out of scope: auto-update, MSIX/Burn bootstrapper, commercial code-signing enrollment, Mica/acrylic, a shared Rust/macOS ABI, a full custom UI Automation tree, and publication without an explicit request.
