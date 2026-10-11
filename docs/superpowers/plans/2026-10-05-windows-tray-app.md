# Windows Tray App Implementation Plan

> **Status: superseded.** Use [the current approved Windows implementation plan](../../changes/windows-tray-app.md), updated on 2026-10-10 for the current macOS UI and WiX 7. The task details below are retained as historical planning notes, including their older WiX 5 assumptions; do not execute them as the current plan.

**Goal:** A Windows 11 (x64, arm64) tray app, `ai-usage.exe`, that shows the same provider usage, pixel icon and card panel as the macOS menu bar app, in under 30 MB of memory.

**Architecture:** A Cargo workspace under `windows/`. `ai-usage-core` is a behavior-for-behavior port of `Sources/AIUsageCore/` (pure logic plus the Windows process runner) and is tested with `cargo test`; its tests are ports of `Sources/CoreChecks/`. `ai-usage` is the Win32 app: tray icon, scheduler, context menu and a Direct2D flyout that draws the core's `Card` model. Shipped provider JSON is embedded in the exe.

**Tech Stack:** Rust (stable, edition 2021, at least 1.77), `windows` crate (Win32, Direct2D, DirectWrite, DWM), `serde_json`; build-time only: `embed-resource`, WiX v5 .NET tool, PowerShell build scripts.

**Spec:** `docs/superpowers/specs/2026-10-05-windows-tray-app-design.md`. Behavior not restated in the spec is the macOS behavior in `docs/specs/menubar/*.md`, `docs/specs/data-source/*.md` and the Swift sources named in each task.

## Global Constraints

- Runtime dependencies: `windows` and `serde_json` only. Build-time: `embed-resource`, WiX. No GUI framework, no async runtime, no chrono.
- Memory: under 30 MB, Task Manager "Memory (private working set)".
- Only timer: one 30 s `SetTimer`. No other polling.
- Worker threads run providers, at most 2 at a time; results return to the UI thread by `PostMessageW`.
- Never read, store or log credentials. Never pass the app's `GITHUB_TOKEN`/`GH_TOKEN` to a child.
- Provider format is `example/AGENTS.md` plus this repo's additions; change Swift only if the format itself changes (it does not in this plan).
- Shared `Resources/providers/*.json` is not edited. Windows overrides live in `Resources/providers-windows/`.
- Settings: `%APPDATA%\ai-usage\settings.json`; user providers: `%APPDATA%\ai-usage\providers\`; crash line: `%LOCALAPPDATA%\ai-usage\last-error.txt`.
- Colors: green `RGB(52,199,89)` below 70%, orange `RGB(255,149,0)` from 70%, red `RGB(255,59,48)` from 90%; gray `RGB(110,110,110)`.
- Pin format `run` or `run|meter`; settings keys `pinnedProvider`, `hiddenCards`, `disabledProviders`, `overflowHintDismissed`.
- Port tests first: every core module's Rust tests are written and seen failing before its code.
- Do not edit version numbers by hand except through `windows/scripts/release.ps1`. Do not commit, push or publish unless the human asks; at the end of each task, stop and report so the human can decide.
- Mark anything not checked against a tool's own docs or a real run as unconfirmed.

## Review Focus

- An executable under a path with spaces (`C:\Program Files\GitHub CLI\gh.exe`) or a `.cmd` shim in a folder with spaces: it must be found and started, with its arguments intact. Pinned by `runs_cmd_shim_in_folder_with_spaces` (Task 6).
- The tray icon is in the `^` overflow, so `Shell_NotifyIconGetRect` fails or returns the overflow button, and monitors left of or above the primary have negative coordinates: the flyout must still open fully inside a work area. Pinned by `flyout_position_*` tests (Task 10).
- Child output with a UTF-8 BOM or Windows line endings (PowerShell, some npm shims): JSON must still parse and stdio lines must still match. Pinned by `parse_strips_utf8_bom` (Task 1) and `stdio_accepts_crlf_lines` (Task 6).
- A settings file that is empty, truncated, or from a crash mid-write: the app must start with defaults and rewrite it, and saving must never leave a half-written file. Pinned by `settings_*` tests (Task 8).
- A four-character value such as `100%` is 15 font pixels wide, wider than the 11 px the spec assumes: the icon must pick a smaller scale rather than clip. Pinned by `tray_scale_*` tests (Task 3).

---

### Task 0: Toolchain check, workspace, repo rules

The human installs the prerequisites (spec, "Build, install, release"). This task only checks them.

**Files:**
- Create: `windows/Cargo.toml` (workspace, members `core`, `app`, `resolver = "2"`, release profile `opt-level = "s"`, `lto = true`, `codegen-units = 1`; keep the default `panic = "unwind"`)
- Create: `windows/.cargo/config.toml` (`[target.x86_64-pc-windows-msvc]` and `[target.aarch64-pc-windows-msvc]` with `rustflags = ["-C", "target-feature=+crt-static"]`)
- Create: `windows/core/Cargo.toml` (package `ai-usage-core`, lib), `windows/core/src/lib.rs`
- Create: `windows/app/Cargo.toml` (package `ai-usage`, bin name `ai-usage`), `windows/app/src/main.rs` (empty `fn main() {}` for now)
- Modify: `.gitignore` (add `windows/target/`, `windows/dist/`)
- Modify: `AGENTS.md`, `docs/specs/tech-stack.md`

- [ ] **Step 1: Check prerequisites**

Run: `rustc --version; cargo --version; rustup target list --installed`
Expected: rustc ≥ 1.77; both `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` listed. If not, stop and ask the human to install them; do not install system software yourself.

- [ ] **Step 2: Create the workspace files above and build both targets**

Run (from `windows/`): `cargo build --target x86_64-pc-windows-msvc; cargo build --target aarch64-pc-windows-msvc; cargo test`
Expected: all succeed (0 tests). If arm64 linking fails, report the exact error to the human (likely missing MSVC ARM64 tools).

- [ ] **Step 3: Update repo rules**

`AGENTS.md`: replace "This is one native macOS app…" with: the macOS app is Swift/AppKit (unchanged rules), the Windows app is Rust/Win32 under `windows/` (rules: runtime deps `windows` + `serde_json` only, logic in `windows/core` tested by `cargo test` with tests first, memory under 30 MB in Task Manager "Memory (private working set)", only the 30 s timer). Keep "do not bring back the old Rust core" and say the Windows code is new.
`docs/specs/tech-stack.md`: add a "Windows" section with the same facts and the two crates.

---

### Task 1: JSON helpers and provider config

Port of `Sources/AIUsageCore/JSON.swift` and `ProviderConfig.swift`.

**Files:**
- Create: `windows/core/src/json.rs`, `windows/core/src/config.rs`
- Test: in-file `#[cfg(test)] mod tests`
- Test fixtures: `include_str!("../../../Resources/providers/<id>.json")`

**Interfaces:**
- Produces (`json.rs`, over `serde_json::Value`):
  - `pub fn parse(bytes: &[u8]) -> Option<Value>` (strips a leading UTF-8 BOM, accepts bare values)
  - `pub fn resolve<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value>` (RFC 6901, leading `/`s dropped as in Swift; `""` = root)
  - `pub fn number(v: Option<&Value>) -> Option<f64>`, `pub fn bool(v: Option<&Value>) -> Option<bool>`, `pub fn is_null(v: Option<&Value>) -> bool`, `pub fn id_string(v: Option<&Value>) -> Option<String>`, `pub fn equal(a: &Value, b: &Value) -> bool`, `pub fn line(v: &Value) -> Option<Vec<u8>>` (one-line, slashes not escaped)
- Produces (`config.rs`): `ProviderConfig { id, name, icon_label, icon_color: Rgb, source: Source, map: MapSpec, refresh_seconds: u32, accounts: Option<AccountsSpec> }`, `Rgb { r, g, b }` with `Rgb::from_hex(&str) -> Option<Rgb>` and `Rgb::GRAY`, `Source::{Command(CommandSource), Stdio(StdioSource)}` with `replacing_account(&self, &str) -> Source` and `executable(&self) -> &str`, `Step::{Write(Vec<u8>), Await(Expect)}`, `Expect`, `Predicate { path, test: Test::{Equals(Value), NotEquals(Value), Exists(bool)} }` with `Predicate::all_hold(&[Predicate], &Value) -> bool`, `MapSpec`, `PlanSpec`, `Scope`, `IdSpec`, `LabelSpec`, `MeterSpec`, `WindowSpec`, `UsedAs`, `ResetAs`, `DurationAs`, `DurationSpec`, and `ProviderConfig::parse(bytes: &[u8]) -> Result<ProviderConfig, ConfigError>`; `ConfigError` displays as `"<json pointer>: <message>"` (`"/"` for the root).
- Field names, defaults, ranges and error texts: exactly as `ProviderConfig.swift` (timeouts 30/15, ranges `1...600`, `1024...8_388_608`, `maxTotalBytes` default `max(4_194_304, maxLine)`, refresh clamped `60...86_400` default 300, id regex `^[a-z0-9][a-z0-9-]{0,39}$` checked by hand, no regex crate).

- [ ] **Step 1: Port the tests** — every check in `Sources/CoreChecks/ModelChecks.swift` `jsonChecks()` and `configChecks()` (lines 4–50) becomes a `#[test]` with the same name in snake_case and the same values (`pointer_nested`, `equal_int_and_float`, `bundled_icon_labels` = `["AGY","CLD","CDX","GHC"]`, `error_names_arg_path` = `"/source/args/1: expected a string"`, …). Add:

```rust
#[test]
fn parse_strips_utf8_bom() {
    let v = parse(b"\xEF\xBB\xBF{\"a\":1}").unwrap();
    assert_eq!(number(resolve(&v, "/a")), Some(1.0));
}
```

- [ ] **Step 2: Run** `cargo test -p ai-usage-core` — Expected: FAIL to compile (functions missing).
- [ ] **Step 3: Implement `json.rs` and `config.rs`** — a private `Reader { value: &Value, path: String }` mirroring the Swift `Reader` keeps error paths identical.
- [ ] **Step 4: Run** `cargo test -p ai-usage-core` — Expected: all PASS.

---

### Task 2: Report and mapping

Port of `Report.swift`.

**Files:**
- Create: `windows/core/src/report.rs`, `windows/core/src/time.rs`

**Interfaces:**
- Consumes: Task 1 types.
- Produces: `Timestamp = f64` (Unix seconds); `Report { plan: Option<String>, available: Option<bool>, access: Option<bool>, meters: Vec<MeterReport> }` with `max_percent()`, `headline_percent()`; `MeterReport { id, label, windows: Vec<WindowReport> }` with `max_percent()`; `WindowReport { id, label: Option<String>, used_percent: Option<f64>, resets_at: Option<Timestamp>, duration_seconds: Option<i64> }` with `kind() -> WindowKind`; `WindowKind::{Session, Weekly, Other}` (ordered); `ProviderFailure(pub String)`; `mapper::apply(&MapSpec, &Value) -> Report`; `convert::{used_percent, date, duration_seconds, duration_label}`; `time::parse_rfc3339(&str) -> Option<Timestamp>` (Z or ±hh:mm offset, fraction of any length; days-from-civil arithmetic, no crate).

- [ ] **Step 1: Port the tests** — `mappingChecks()` in `ModelChecks.swift` (lines 52–143) one `#[test]` per check, same answers and expected values. Add `rfc3339_fraction_of_any_length` (`"2026-10-05T08:20:00.123456789Z"` → `1791102000.123456789` within 1e-6), `rfc3339_offset` (`"2026-10-05T15:20:00+07:00"` equals `"2026-10-05T08:20:00Z"`), `rfc3339_rejects_garbage`.
- [ ] **Step 2: Run** `cargo test -p ai-usage-core report` — Expected: FAIL.
- [ ] **Step 3: Implement** — `eachEntry` sorts keys, duplicate meter/window ids keep the first, a window with neither used nor reset is skipped, exactly as Swift.
- [ ] **Step 4: Run** `cargo test -p ai-usage-core` — Expected: PASS.

---

### Task 3: Pixel icon, status icons, pin, relative time

Port of `PixelIcon.swift`, `RelativeTime.swift`.

**Files:**
- Create: `windows/core/src/icon.rs`, `windows/core/src/pin.rs`, `windows/core/src/run.rs`; extend `time.rs`

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces:
  - `run.rs`: `ProviderRun { id, name, icon_label, icon_color: Rgb, result: Option<Result<Report, ProviderFailure>>, config_id, provider_name, account: Option<String> }`, `ProviderRun::from_config(&ProviderConfig) -> ProviderRun`.
  - `icon.rs`: `IconSpec { top, bottom, top_color, bottom_color, accessibility }`; `pixel_layout(top: &str, bottom: &str) -> PixelLayout { width, height, pixels: Vec<Pixel { x, y, bottom_row }> }` (same 3×5 glyph table, copy it verbatim from `PixelIcon.swift`); `status_icon(runs: &[ProviderRun], pinned: Option<&str>) -> IconSpec`; `severity(f64) -> Rgb`; `percent_text(f64) -> String` (`format!("{:.0}%")`); `meter_label(&str) -> String`; `PLACEHOLDER` (`AI`/`--`, accessibility `AI Usage`).
  - `icon.rs`: `tray_scale(icon_px: u32, layout_w: u32, layout_h: u32) -> u32` (largest integer ≥ 1 with `w*s <= icon_px && h*s <= icon_px`, else 1) and `render_rgba(spec: &IconSpec, icon_px: u32) -> Vec<u8>` (premultiplied BGRA, `icon_px²*4` bytes, layout centered, transparent elsewhere).
  - `pin.rs`: `pin_key(run, meter) -> String`, `resolve(pin, runs) -> Option<(&ProviderRun, Option<String>)>`, `effective(pin, runs) -> Option<String>`.
  - `time.rs`: `relative_text(until: Timestamp, now: Timestamp) -> Option<String>`; `LocalTime { year, month, day, hour, minute }` and `format_reset(&LocalTime) -> String` = `"Oct 5, 3:20 PM"` (en-US month abbreviations, 12-hour, `AM`/`PM`).

- [ ] **Step 1: Port the tests** — `iconChecks()` (lines 157–200) and `pinChecks()` (234–271) and the relative-time checks in `cardChecks()` (`relative(...)`, from line 317). Add:

```rust
#[test] fn tray_scale_three_letters() { assert_eq!(tray_scale(16, 11, 11), 1); assert_eq!(tray_scale(24, 11, 11), 2); assert_eq!(tray_scale(32, 11, 11), 2); }
#[test] fn tray_scale_hundred_percent() { assert_eq!(tray_scale(16, 15, 11), 1); assert_eq!(tray_scale(24, 15, 11), 1); assert_eq!(tray_scale(32, 15, 11), 2); }
#[test] fn render_rgba_size_and_transparent_corner() { let b = render_rgba(&PLACEHOLDER, 16); assert_eq!(b.len(), 16*16*4); assert_eq!(&b[0..4], &[0,0,0,0]); }
#[test] fn format_reset_afternoon() { assert_eq!(format_reset(&LocalTime{year:2026,month:10,day:5,hour:15,minute:20}), "Oct 5, 3:20 PM"); }
#[test] fn format_reset_midnight() { assert_eq!(format_reset(&LocalTime{year:2026,month:1,day:9,hour:0,minute:5}), "Jan 9, 12:05 AM"); }
```
- [ ] **Step 2: Run** `cargo test -p ai-usage-core` — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p ai-usage-core` — Expected: PASS.

---

### Task 4: Card model, menu model, schedule

Port of `CardModel.swift`, `MenuModel.swift`.

**Files:**
- Create: `windows/core/src/card.rs`, `windows/core/src/menu.rs`

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces:
  - `card.rs`: `Tone::{Normal, Secondary, Green, Orange, Red}`, `CardRing`, `CardRow`, `CardChart`, `CardNotice`, `Card` (fields as Swift, snake_case), `is_grouped(&Report) -> bool`, `title_pin(&ProviderRun) -> String`, `card(run: &ProviderRun, pinned: Option<&str>, now: Timestamp, format_reset: &dyn Fn(Timestamp) -> String) -> Card`. `MAX_RINGS = 4`, `MAX_CHARTS = 4`. Text `Loading…`, `Error: <msg>`, `Usage limit reached`, `Plan limits don't apply to this account`, `No usage data`, `resetting…`, separator ` · ` exactly as Swift.
  - `menu.rs`: `Panel { cards: Vec<Card>, notices: Vec<CardNotice>, updated: Option<String> }` and `panel(runs, pinned, config_errors, providers: &[ProviderToggle], hidden: &HashSet<String>, updated: Option<String>, now, format_reset) -> Panel` (the flyout half of `MenuModel.entries`: cards for shown runs, `All providers are off`, `All cards are hidden`, config errors in red); `ProviderToggle { id, name, enabled }` with `visible(runs, disabled) -> Vec<ProviderRun>`; `HiddenCard { id, name }` and `hidden_cards(runs, hidden) -> Vec<HiddenCard>`; `schedule_delay(interval: u32, failures: u32) -> f64` (seconds: interval if 0 failures, else `min(60 * 2^(min(f,6)-1), 1800)`).

- [ ] **Step 1: Port the tests** — rest of `cardChecks()` (from line 317), `menuChecks()` (201–233) and `toggleChecks()` (272–316), translated to `panel`/`hidden_cards` (checks about menu items that move to the context menu — Providers, Hidden Cards, Launch at Login, version — are covered in Task 9, not here). Add `schedule_backoff`: `[0,1,2,3,6,7,20]` failures with interval 300 → `[300,60,120,240,1800,1800,1800]`.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p ai-usage-core` — Expected: PASS.

---

### Task 5: Executable lookup and child environment

Windows replacement for `ChildEnvironment.swift`. Pure functions; registry reading is in the app (Task 8) and passed in.

**Files:**
- Create: `windows/core/src/env.rs`

**Interfaces:**
- Produces:
  - `child_path(machine_path: &str, user_path: &str, userprofile: &str, appdata: &str) -> String` (machine; user; `%USERPROFILE%\.local\bin`; `%APPDATA%\npm`; empty and duplicate entries removed case-insensitively; inputs already expanded).
  - `find_executable(name: &str, path: &str, pathext: &str, exists: &dyn Fn(&Path) -> bool) -> Option<PathBuf>` (absolute or containing `\`/`/`: checked as is, then with each extension; else each `PATH` dir × (name as is if it has an extension in `PATHEXT`, then name + each `PATHEXT` ext); first hit wins; `PATHEXT` default `.COM;.EXE;.BAT;.CMD`).
  - `child_variables(parent: &HashMap<String,String>, path: String, overrides: &HashMap<String,String>) -> HashMap<String,String>` (pass list from the spec, case-insensitive names, `TERM=dumb`, `NO_COLOR=1`, `HOME`=`USERPROFILE`, `PATH`=`path`, then overrides; `GH_TOKEN`/`GITHUB_TOKEN` never from parent, but allowed from overrides).

- [ ] **Step 1: Write the tests**

```rust
#[test] fn finds_cmd_shim_via_pathext() // dirs ["C:\\a","C:\\npm"], exists only C:\npm\codex.CMD → Some("C:\\npm\\codex.CMD")
#[test] fn exe_before_cmd_in_same_dir() // both gh.EXE and gh.CMD exist → .EXE (PATHEXT order)
#[test] fn earlier_dir_wins()
#[test] fn name_with_extension_used_as_is() // "powershell.exe"
#[test] fn absolute_path_checked_directly()
#[test] fn not_found_is_none()
#[test] fn child_path_order_and_dedupe() // user dir equal to machine dir differing in case appears once
#[test] fn env_drops_parent_tokens() // parent GH_TOKEN, GITHUB_TOKEN, gh_token → absent
#[test] fn env_keeps_allowed_and_sets_fixed() // SystemRoot, https_proxy kept; TERM=dumb; NO_COLOR=1; HOME=USERPROFILE value
#[test] fn env_drops_unlisted() // RANDOM_SECRET absent
#[test] fn env_overrides_win() // override GH_TOKEN=x present
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p ai-usage-core env` — Expected: PASS.

---

### Task 6: Subprocess with Job Object, and the runner

Port of `Subprocess.swift` and `ProviderRunner.swift`.

**Files:**
- Create: `windows/core/src/process.rs`, `windows/core/src/runner.rs`
- Create: `windows/core/tests/fixtures/` (small `.cmd` scripts), `windows/core/tests/runner.rs`
- Modify: `windows/core/Cargo.toml` (add `windows` with features `Win32_Foundation`, `Win32_System_JobObjects`, `Win32_System_Threading`)

**Interfaces:**
- Consumes: Tasks 1, 2, 3 (`ProviderRun`), 5.
- Produces:
  - `process.rs`: `Child::spawn(exe: &Path, args: &[String], env: &HashMap<String,String>, dir: &Path, pipe_stdin: bool) -> io::Result<Child>` using `std::process::Command` with `CREATE_NO_WINDOW` (`0x0800_0000`) and `env_clear()`, stderr null, then assigned to a new Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; a reader thread feeds stdout into a channel. Methods `read_all(deadline: Instant, max: usize) -> ReadAll::{Data(Vec<u8>), Timeout, TooMuch}`, `read_line(deadline, max_line, max_total) -> ReadLine::{Line(Vec<u8>), Eof, Timeout, TooLong, TooMuch}` (strips trailing `\r\n` or `\n`), `write(&[u8]) -> bool`, `close_stdin()`, `wait(deadline) -> Option<i32>`, and `Drop` closes the job (kills the tree).
  - `runner.rs`: `Runner { path: String, parent_env: HashMap<String,String> }` with `run(&ProviderConfig) -> Result<Report, ProviderFailure>`, `run_all(&ProviderConfig) -> Vec<ProviderRun>`, `capture(&Source) -> Result<Value, ProviderFailure>`. Error texts exactly as `ProviderRunner.swift` (`"<name> not found on PATH"`, `"<name> timed out"`, `"<name> printed more than N bytes"`, `"<name> exited with status N"`, `"<name> output was not valid JSON"`, `"could not list accounts: …"`, `"no accounts found"`, `"Update the CLI to see usage"` for code -32601, …). Account labels `GH1`…`GH9` rule as Swift. Each capture runs in a new `%TEMP%\ai-usage-<random>` folder removed afterwards.

- [ ] **Step 1: Port the tests** — `runnerChecks()` and `accountChecks()` in `RunnerChecks.swift` (56–155) become integration tests in `tests/runner.rs`, with each `/bin/sh` script rewritten as a `.cmd` fixture (`@echo off` + `echo {...}`; slow ones use `ping -n 30 127.0.0.1 >nul`). The tree test's fixture starts `cmd /c ping -n 61 127.0.0.1 >nul` as a grandchild; the test counts `PING.EXE` lines in `tasklist /FI "IMAGENAME eq PING.EXE"` before the run and 2 s after the timeout, and expects the same count. `environmentChecks()` (4–31) is already covered by Task 5. Add:

```rust
#[test] fn runs_cmd_shim_in_folder_with_spaces() // copy fixture to "<tmp>\\dir with space\\answer.cmd", PATH = that dir, args ["a b", "c"]; script echoes JSON with %1; expect success and the arg intact
#[test] fn stdio_accepts_crlf_lines() // fixture answers with CRLF lines; await matches
#[test] fn timeout_kills_grandchild()
#[test] fn parent_gh_token_not_visible_to_child() // fixture prints {"t":"%GH_TOKEN%"}; parent env has GH_TOKEN=secret → output "t" is "%GH_TOKEN%" (unset in cmd prints literally)
```
- [ ] **Step 2: Run** `cargo test -p ai-usage-core --test runner` — Expected: FAIL.
- [ ] **Step 3: Implement `process.rs`, then `runner.rs`.** Job assignment happens right after spawn; a child that exits before assignment is fine. Unconfirmed until this test passes: that `std::process::Command` quoting of `"a b"` for a `.cmd` target reaches `%1` as `"a b"` (Rust ≥ 1.77 behavior).
- [ ] **Step 4: Run** `cargo test -p ai-usage-core` — Expected: PASS, and no stray `PING.EXE` afterwards.

---

### Task 7: Provider store and the Windows Copilot override

Port of `ProviderStore.swift` plus embedded files.

**Files:**
- Create: `windows/core/src/store.rs`
- Create: `Resources/providers-windows/copilot.json`
- Create: `docs/specs/windows/runner.md` (Windows runner rules as implemented: lookup, env list, Job Object, PATH sources, load order, copilot override)

**Interfaces:**
- Consumes: Tasks 1, 6.
- Produces: `EMBEDDED: &[(&str, &str)]` (file name, contents) built with `include_str!` from `Resources/providers/*.json` then `Resources/providers-windows/*.json`; `load(embedded: &[(&str,&str)], user_folder: &Path) -> (Vec<ProviderConfig>, Vec<String>)` (later same id replaces earlier; user files: regular files only, not symlinks, ≤ 256 KB, `.json`, id must equal file stem; errors `"<file>: <error>"`; result sorted by id); `user_folder(appdata: &Path) -> PathBuf` = `<APPDATA>\ai-usage\providers`.

- [ ] **Step 1: Verify the PowerShell command by hand (unconfirmed item from the spec)**

The override passes the account through an environment variable (the format already replaces `${account}` in `env`), so no quoting of the account into script text is needed. Source:

```json
"source": {
  "type": "command",
  "executable": "powershell.exe",
  "args": ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command",
           "[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); $t = gh auth token --user $env:AI_USAGE_ACCOUNT; if ($LASTEXITCODE -ne 0 -or -not $t) { exit 1 }; $env:GH_TOKEN = $t; gh api copilot_internal/user; exit $LASTEXITCODE"],
  "env": { "AI_USAGE_ACCOUNT": "${account}" },
  "timeoutSeconds": 20,
  "expect": { "require": "/quota_snapshots" }
}
```
Everything else is copied from `Resources/providers/copilot.json` (`revision` +1). Run by hand with your own login: `$env:AI_USAGE_ACCOUNT='<login>'; powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "<the script>" | Out-File -Encoding utf8 $env:TEMP\cp.json` and check it is the JSON with `quota_snapshots`. Ask the human to run it if `gh` is not logged in here. Record the result (confirmed or not) in `docs/specs/windows/runner.md`.

- [ ] **Step 2: Write the tests**

```rust
#[test] fn embedded_loads_four_providers() // ids ["antigravity","claude","codex","copilot"], no errors
#[test] fn windows_override_replaces_copilot() // copilot source executable == "powershell.exe"
#[test] fn user_file_replaces_embedded() // tmp folder with claude.json name "Mine"
#[test] fn user_file_id_must_match_name() // error "x.json: /id: must match the file name"
#[test] fn user_file_too_big_rejected() // 257 KB → "only regular files up to 256 KB load"
#[test] fn bad_json_listed_not_fatal()
```
- [ ] **Step 3: Run** — Expected: FAIL.
- [ ] **Step 4: Implement `store.rs` and add the JSON file.**
- [ ] **Step 5: Run** `cargo test -p ai-usage-core` — Expected: PASS.

---

### Task 8: App skeleton — tray icon, scheduler, settings, `--live`

**Files:**
- Create: `windows/app/src/main.rs`, `windows/app/src/tray.rs`, `windows/app/src/scheduler.rs`, `windows/app/src/settings.rs`, `windows/app/src/system.rs`
- Create: `docs/specs/windows/tray-icon.md`
- Modify: `windows/app/Cargo.toml` (`#![windows_subsystem = "windows"]` in main; `windows` features `Win32_UI_WindowsAndMessaging`, `Win32_UI_Shell`, `Win32_Graphics_Gdi`, `Win32_UI_HiDpi`, `Win32_System_Registry`, `Win32_System_Threading`, `Win32_System_Environment`, `Win32_System_Time`, `Win32_Foundation`)

**Interfaces:**
- Consumes: all core modules.
- Produces:
  - `settings.rs`: `Settings { pinned_provider: Option<String>, hidden_cards: Vec<String>, disabled_providers: Vec<String>, overflow_hint_dismissed: bool }`, `Settings::load(path) -> Settings` (missing, empty, invalid → default), `Settings::save(&self, path) -> io::Result<()>` (write `settings.json.tmp`, then `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`).
  - `system.rs`: `registry_paths() -> (String, String)` (HKLM `SYSTEM\CurrentControlSet\Control\Session Manager\Environment\Path` and HKCU `Environment\Path`, `REG_EXPAND_SZ` expanded with `ExpandEnvironmentStringsW`), `to_local(Timestamp) -> LocalTime` (`SystemTimeToTzSpecificLocalTime`), `apps_use_light_theme() -> bool`, `system_uses_light_theme() -> bool`, `known_folder_appdata() / localappdata()`.
  - `scheduler.rs`: `Scheduler` owning configs, runs (`Vec<ProviderRun>` in config order), per-config state (next, failures, running) with `tick(hwnd)`, `refresh_now(hwnd)` (rereads registry PATH and provider files), `finish(config_id, Vec<ProviderRun>)`; workers are `std::thread`s gated by a counter ≤ 2; results posted as `WM_APP + 1` with a boxed payload pointer in `lParam`.
  - `tray.rs`: `Tray::add(hwnd, &IconSpec, dpi)`, `update(&IconSpec)`, `re_add()` on `TaskbarCreated`, `icon_rect() -> Option<RECT>`; HICON via `CreateIconIndirect` from `render_rgba` at `GetSystemMetricsForDpi(SM_CXSMICON, dpi)`; tooltip = `accessibility` (max 127 chars). Gray parts use `RGB(110,110,110)` on dark taskbar, `RGB(90,90,90)` on light (`system_uses_light_theme`).
  - `main.rs`: named mutex `Local\AIUsage.SingleInstance` (exit if it exists); panic hook writes `"<unix seconds> <panic message>"` (one line, overwrite) to `last-error.txt`; hidden message-only-style top-level window (not `HWND_MESSAGE`, so `TaskbarCreated` arrives); 30 s `SetTimer`; `WM_DPICHANGED`/`WM_SETTINGCHANGE` redraw the icon. `ai-usage.exe --live`: attaches to the parent console (`AttachConsole(ATTACH_PARENT_PROCESS)`), runs every provider once synchronously, prints each card's title, rows and reset lines, exits 0.

- [ ] **Step 1: Write the settings tests** (in `settings.rs`)

```rust
#[test] fn settings_missing_file_is_default()
#[test] fn settings_empty_file_is_default()
#[test] fn settings_truncated_json_is_default() // "{\"pinnedProvider\": \"cl"
#[test] fn settings_round_trip()
#[test] fn settings_save_leaves_no_tmp_file()
```
- [ ] **Step 2: Run** `cargo test -p ai-usage` — Expected: FAIL.
- [ ] **Step 3: Implement the files above.**
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: PASS.
- [ ] **Step 5: Run it** — `cargo run -p ai-usage -- --live` prints a block per provider (failures show their error text). Then `cargo run -p ai-usage --release`: the tray icon appears (maybe in `^`), tooltip names the pinned provider, value updates after the first run; a second launch exits; `taskkill /IM explorer.exe /F; start explorer` brings the icon back.
- [ ] **Step 6: Write `docs/specs/windows/tray-icon.md`** — the icon as implemented (sizes, scales, colors, tooltip, overflow note), linking to `docs/specs/menubar/menubar-icon.md` for shared rules.

---

### Task 9: Context menu

**Files:**
- Create: `windows/app/src/context_menu.rs`, `windows/app/src/login.rs`
- Modify: `windows/app/src/main.rs` (`WM_CONTEXTMENU`/`NIN_*` from the tray callback with `NOTIFYICON_VERSION_4`)

**Interfaces:**
- Consumes: Task 4 `ProviderToggle`, `hidden_cards`; Task 8 `Settings`, `Scheduler`.
- Produces: `MenuItem` enum and pure `menu_items(providers: &[ProviderToggle], hidden: &[HiddenCard], launch_at_login: bool, version: &str) -> Vec<MenuItem>` in that file (order: Refresh Now, separator, Providers ▸, Hidden Cards ▸ only when non-empty, Launch at Login (checked), Open Providers Folder…, separator, `AI Usage <version>` disabled, Quit); `show(hwnd, items, point)` with `TrackPopupMenuEx` after `SetForegroundWindow`; `login::{is_enabled() -> bool, set(enabled: bool, exe: &Path) -> io::Result<()>}` on `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value `AI Usage` = `"<exe path>"` (quoted).

- [ ] **Step 1: Write the tests** — `menu_items_order`, `hidden_cards_submenu_only_when_hidden`, `providers_submenu_checks_match_enabled`, `version_item_disabled_text` (`"AI Usage 0.1.0"`).
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.** Toggling a provider saves `disabledProviders` and calls `tick`; Show <name> removes it from `hiddenCards`; Open Providers Folder creates the folder then `ShellExecuteW("open")`; Quit removes the tray icon and posts `WM_QUIT`.
- [ ] **Step 4: Run** `cargo test --workspace` — PASS. Manual: each item works; Launch at Login survives a restart of the app (check mark matches the registry).

---

### Task 10: Flyout window shell

The window, its position, theme, closing and footer. Cards are drawn in Task 11.

**Files:**
- Create: `windows/app/src/flyout/mod.rs`, `windows/app/src/flyout/position.rs`, `windows/app/src/flyout/theme.rs`
- Modify: `windows/app/Cargo.toml` (`Win32_Graphics_Direct2D`, `Win32_Graphics_Direct2D_Common`, `Win32_Graphics_DirectWrite`, `Win32_Graphics_Dxgi_Common`, `Win32_Graphics_Dwm`, `Win32_UI_Input_KeyboardAndMouse`)

**Interfaces:**
- Consumes: Task 8 `Tray::icon_rect`, `system::apps_use_light_theme`.
- Produces:
  - `position.rs`: `Rect { left, top, right, bottom: i32 }`; `flyout_position(icon: Option<Rect>, work_area: Rect, monitor: Rect, size: (i32, i32), gap: i32) -> (i32, i32)` — taskbar edge = the side of `monitor` not covered by `work_area` (bottom if none); place the flyout on that side of the icon, centered on the icon along the edge, `gap` px away from the taskbar; with `icon == None` use the work area corner nearest the taskbar on the right (or bottom for a left/right taskbar); always clamp fully inside `work_area`.
  - `theme.rs`: `Palette { background, text, secondary, outline, outline_hover, accent, button_fill_hover, button_fill_pressed }` for light and dark (accent from `DwmGetColorizationColor`).
  - `mod.rs`: `Flyout::toggle(owner, icon_rect)`, `Flyout::close()`; window `WS_POPUP`, ex `WS_EX_TOOLWINDOW | WS_EX_TOPMOST`; rounded corners with `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND`; closes on `WM_ACTIVATE(WA_INACTIVE)` and `Esc`; `F5`/`Ctrl+R` refresh; mouse wheel scroll when content is taller than the work area minus 2×gap; footer `Updated h:mm AM` + Refresh button; one-time overflow hint line with a `×` that sets `overflowHintDismissed`. A click on the tray icon while open closes it (ignore the deactivate-then-click race with a 200 ms guard).

- [ ] **Step 1: Write the position tests**

```rust
#[test] fn flyout_position_bottom_taskbar_above_icon()
#[test] fn flyout_position_clamped_at_right_edge() // icon near screen right → flyout right == work_area.right
#[test] fn flyout_position_top_taskbar_below_icon()
#[test] fn flyout_position_left_taskbar_right_of_icon()
#[test] fn flyout_position_no_icon_rect_uses_corner() // None → bottom-right corner of work area, inside it
#[test] fn flyout_position_negative_monitor_coords() // monitor (-1920,0,0,1080), work (-1920,0,0,1032) → inside that work area
#[test] fn flyout_position_taller_than_work_area_starts_at_top()
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace` — PASS. Manual: empty flyout opens next to the icon (taskbar at bottom; icon in and out of `^`), closes on outside click and `Esc`, switches light/dark live when Windows "app mode" changes.

---

### Task 11: Cards in the flyout

Draw `Panel` with Direct2D/DirectWrite, plus hover, Pin, Hide and keyboard.

**Files:**
- Create: `windows/app/src/flyout/layout.rs`, `windows/app/src/flyout/draw.rs`
- Create: `docs/specs/windows/flyout.md`

**Interfaces:**
- Consumes: Task 4 `Panel`, `Card`; Task 10 `Palette`, `Flyout`.
- Produces:
  - `layout.rs` (pure, unit-tested; text measured through a `Measure` trait so tests use a fake fixed-width measurer): `layout_panel(panel: &Panel, measure: &dyn Measure) -> PanelLayout { width, height, items: Vec<Item> }`, where `Item` is a positioned card outline, text run, ring set, pill, or `Button { kind: Pin(String) | Hide(String), rect, card_id }`. Dimensions in DIPs copied from `Sources/AIUsage/CardView.swift`: min width 300, chart 76, ring thickness 5, ring gap 2, outer gap 4, side gap 10, edge 16, inner pad 12, small 4, medium 8, large 14, line 20, reset line 14, dot column 14, pill height 18, pill padding 7, cards 8 apart; fonts Segoe UI 13 (title, bold when pinned), 12 (body, group bold), 11 (small, buttons, center).
  - `hit_test(layout, x, y) -> Option<Hit::{Card(String), Button(..)}>`.
  - `draw.rs`: draws a `PanelLayout` with a `ID2D1HwndRenderTarget` (recreated on `D2DERR_RECREATE_TARGET`), rings as arcs from 12 o'clock clockwise over a faint track, ring colors = accent blended with white by `[0, 0.3, 0.5, 0.65]`, pill fill = tone color at 16% alpha.

- [ ] **Step 1: Write the layout tests**

```rust
#[test] fn width_is_at_least_300()
#[test] fn width_is_widest_card() // long title → all cards share that width
#[test] fn pin_and_hide_buttons_only_for_hovered_card_except_pinned() // layout takes hovered: Option<&str>
#[test] fn pinned_button_always_present_with_text_pinned()
#[test] fn more_than_four_groups_overflow_as_rows()
#[test] fn hit_test_pin_returns_its_key()
#[test] fn error_card_has_no_rings()
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement `layout.rs`, then `draw.rs`, then wire into `Flyout`** (hover → relayout + `InvalidateRect`; `IDC_HAND` over buttons; click on mouse-up inside the same button → pin or hide, save settings, update tray icon, close flyout; `Tab`/`Shift+Tab` move focus between buttons, `Enter`/`Space` press).
- [ ] **Step 4: Run** `cargo test --workspace` — PASS.
- [ ] **Step 5: Manual check** against `docs/specs/menubar/menubar-panel.md`: Claude, Codex, Copilot (each account), Antigravity groups; Pin moves the icon; Hide moves the card into Hidden Cards; loading and error cards; light and dark; 100% and 150% scale.
- [ ] **Step 6: Write `docs/specs/windows/flyout.md`** — as implemented, linking to the macOS panel spec for shared rules.

---

### Task 12: Build script, resources, arm64, memory

**Files:**
- Create: `windows/scripts/build.ps1`, `windows/app/build.rs`, `windows/app/app.rc`, `windows/app/app.manifest`, `windows/app/assets/app.ico`, `windows/core/examples/make_ico.rs`

**Interfaces:**
- Consumes: Task 3 `pixel_layout`/`render_rgba`.
- Produces: `build.ps1 [-Arch x64|arm64|all]` (default `all`) → `windows/dist/<arch>/ai-usage.exe`; exits non-zero on any failure. `make_ico` writes a multi-size ICO (16, 24, 32, 48, 256; 32-bit BMP entries) of `AI` over `--` in the macOS icon style. `app.manifest`: PerMonitorV2 DPI awareness, Windows 10/11 `supportedOS`, `asInvoker`. `app.rc`: icon + `VERSIONINFO` from `CARGO_PKG_VERSION` (written by `build.rs` through `embed-resource`).

- [ ] **Step 1: Run** `cargo run -p ai-usage-core --example make_ico` — Expected: `windows/app/assets/app.ico` exists; Explorer shows it.
- [ ] **Step 2: Run** `pwsh windows/scripts/build.ps1` — Expected: both exes; Explorer › Properties › Details shows the version; the exe has the icon.
- [ ] **Step 3: Check memory** — run the x64 release exe for 15 minutes with all four providers on, open and close the flyout 10 times; Task Manager "Memory (private working set)" stays under 30 MB. Record the number in `docs/specs/tech-stack.md`. If over: report to the human before optimizing.
- [ ] **Step 4: arm64** — if the human has an arm64 device, they run the arm64 exe and confirm icon, flyout and one provider; otherwise write "arm64 build not run on hardware (unconfirmed)" in `docs/specs/windows/release.md` (Task 13).

---

### Task 13: Installer, release script, README

**Files:**
- Create: `windows/installer/Package.wxs`, `windows/scripts/release.ps1`, `docs/specs/windows/release.md`
- Modify: `README.md` (a "Windows" section: install from MSI or zip, SmartScreen "More info › Run anyway", turning on the tray icon in Settings, building, `--live`)

**Interfaces:**
- Consumes: Task 12 `build.ps1` outputs.
- Produces: `release.ps1 <version> [-Publish]`.

- [ ] **Step 1: Check WiX facts against the WiX v5 docs (unconfirmed items in the spec)** — per-user package attributes (`Scope="perUser"` on `Package`, install folder under `LocalAppDataFolder\Programs`), `MajorUpgrade`, a Start menu shortcut, removing the `HKCU\…\Run` value `AI Usage` on uninstall, and how the installer handles a running `ai-usage.exe` (Restart Manager vs. a "close the app" prompt). Write what the docs say, with links, in `docs/specs/windows/release.md`; mark anything not found as unconfirmed.
- [ ] **Step 2: Write `Package.wxs`** — fixed `UpgradeCode` (generate one GUID once and keep it), `Version` from a `-d Version=` define, `Platform` from `-arch`.
- [ ] **Step 3: Build** `wix build windows/installer/Package.wxs -arch x64 -d Version=0.1.0 -d Exe=windows/dist/x64/ai-usage.exe -o windows/dist/AIUsage-win-0.1.0-x64.msi` (and arm64) — Expected: MSI files.
- [ ] **Step 4: Manual install cycle** (no admin prompt expected): install 0.1.0 → app in Start menu, runs; build 0.1.1 and install over it → one entry in Settings › Apps, version 0.1.1, settings kept; uninstall → files and Run value gone, `%APPDATA%\ai-usage` kept.
- [ ] **Step 5: Write `release.ps1`** — refuses unless on `main` with a clean tree; sets `version` in `windows/app/Cargo.toml` (and `Cargo.lock`); `cargo test --workspace`; `build.ps1 -Arch all`; both MSIs; zips `AIUsage-win-<v>-<arch>.zip` containing `ai-usage.exe`; `SHA256SUMS` (`Get-FileHash -Algorithm SHA256`, format `<hex>  <file>`); if anything fails before the commit, restore `Cargo.toml`/`Cargo.lock`; commits `release: windows <v>` and tags `win-v<v>`. `-Publish`: `git push origin main win-v<v>` and `gh release create win-v<v> <assets> --title "AI Usage for Windows <v>"`. Without `-Publish` nothing leaves the machine.
- [ ] **Step 6: Dry run** on a throwaway branch copy: `release.ps1 0.1.0` stops with the "must be on main" error; on a scratch clone's `main` it produces all five assets. Do not run with `-Publish` unless the human asks.
- [ ] **Step 7: README and `release.md`** — the manual checklist from the spec (DPI 100/125/150/200, light/dark, Explorer restart, two monitors, memory, install/upgrade/uninstall, arm64) with each item's last result.
