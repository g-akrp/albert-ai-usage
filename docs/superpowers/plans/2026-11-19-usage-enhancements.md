# Usage Enhancements Implementation Plan

> **For agentic workers:** Mode C (**Orca workers** via `agent-task-routing` + `orchestration`). Each task runs in a supervised Orca worker from `usage-enhancements` with its routed tier, model, and effort below. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship six high-signal enhancements on base branch `usage-enhancements`: filter hidden cards from menu bar cycling, auto-refresh when a window's `resetsAt` passes, draw window pacing ticks on rings, add a "Most Urgent" menu bar mode, retain last-known usage on transient errors with per-card retry, and expose `--json` CLI output plus richer MCP snapshot fields.

**Architecture:** All domain logic lives in `Sources/AIUsageCore/` with offline checks written first in `Sources/CoreChecks/`. The AppKit shell (`Sources/AIUsage/`) stays thin—no new timers, no third-party dependencies, and `scripts/make-panel.swift` keeps compiling via defaulted initializer parameters.

**Tech Stack:** Swift, Foundation, AppKit only. Verification via `swift run CoreChecks` (no XCTest) and `footprint` memory check.

**Base Branch:** `usage-enhancements` (branched from `main` at `b65a1b2`).

**Orchestration Mode:** `low-cost` (default; human may switch to `balance`, `intelligent`, or `fastest` at the gate—re-picks settings inside each tier, never changes tiers).

## Live Quota Routing (`usage-router` snapshot 2026-11-19)

- **Primary agent:** `antigravity` — 71% tightest (`3p-5h`), 88% `gemini-5h`, 87% `gemini-weekly` (fresh, `stale: false`). Pass `--model <id>` only (`agy` encodes effort in the model ID).
- **Fallback / cross-model reviewer:** `codex` — 28% `Session` (resets `2026-11-19T15:24:57Z`), 89% `Weekly`. Pass `--model <id> --effort <level>`.
- **Avoid:** `claude` — 18% `Weekly` (under 20% floor, resets `2026-11-21T00:00:00Z`). `copilot` — ignores `--model`/`--effort` in Orca.

## Orca Worker Routing Matrix (`low-cost` mode)

| Task | Wave | Depends on | Rubric (A/D/S/R/V) | Tier | Primary (`antigravity`) | Fallback (`codex`) | Escalate (+1 tier) | Worktree |
| - | - | - | - | - | - | - | - | - |
| `t1` Cycle hidden filter | 1 (parallel) | — | `0/0/1/0/0` = 1 | **T1** | `gemini-4-hydrogen-low` | `gpt-6.1-terra` (`low`) | `gemini-4-hydrogen-medium` (T2) | `new-child` (`agent-t1`) |
| `t2` Auto-refresh on reset | 1 (parallel) | — | `0/1/1/1/0` = 3 | **T2** | `gemini-4-hydrogen-medium` | `gpt-6.1-terra` (`medium`) | `gemini-4-hydrogen-high` (T3) | `new-child` (`agent-t2`) |
| `t3` Ring pacing / elapsed | 1 (parallel) | — | `0/1/1/0/1` = 3 | **T2** | `gemini-4-hydrogen-medium` | `gpt-6.1-terra` (`medium`) | `gemini-4-hydrogen-high` (T3) | `new-child` (`agent-t3`) |
| `t4` Most Urgent mode | 2 | `t1` | `0/1/2/1/0` = 4 | **T2** | `gemini-4-hydrogen-medium` | `gpt-6.1-terra` (`medium`) | `gemini-4-hydrogen-high` (T3) | `new-child` (`agent-t4`) |
| `t5` Stale error + retry | 3 | `t2`, `t3`, `t4` | `1/2/2/1/1` = 7 | **T3** | `gemini-4-hydrogen-high` | `gpt-6.1-sol` (`medium`) | `gpt-6.2-sol` (`xhigh`, T4) | `new-child` (`agent-t5`) |
| `t6` `--json` + MCP + README | 4 | `t3`, `t5` | `0/1/1/1/0` = 3 | **T2** | `gemini-4-hydrogen-medium` | `gpt-6.1-terra` (`medium`) | `gemini-4-hydrogen-high` (T3) | `new-child` (`agent-t6`) |
| `t7` Whole-branch review | 5 | `t6` | Reviewer floor (`>= t5`) | **T3** | `gpt-6.1-sol` (`medium`, `codex` cross-model) | `gemini-4-hydrogen-high` (`antigravity`) | `gpt-6.2-sol` (`xhigh`, T4) | `current` (read-only) |

### Alternate Mode Picks (if switched at Gate)

| Mode | T1 (`antigravity` / `codex`) | T2 (`antigravity` / `codex`) | T3 (`antigravity` / `codex`) |
| - | - | - | - |
| `balance` | `gemini-4-hydrogen-low` / `gpt-6.1-terra` (`low`) | `gemini-4-hydrogen-medium` / `gpt-6.2-sol` (`low`) | `gemini-4-hydrogen-high` / `gpt-6.2-sol` (`high`) |
| `intelligent` | `claude-opus-5-5-low` / `gpt-6.1-sol` (`low`) | `claude-sonnet-5-5-high` / `gpt-6-sol` (`max`) | `gemini-4-hydrogen-xhigh` / `gpt-6.2-sol` (`high`) |
| `fastest` | `gemini-4-hydrogen-low` / `gpt-6.1-terra` (`low`) | `gemini-4-hydrogen-medium` / `gpt-6.2-sol` (`low`) | `claude-opus-5-5-medium` / `gpt-6.2-astra` (`low`) |

### Orca Dispatch Protocol

1. **Gate first:** Human approves this plan, orchestration mode, and `new-child` worktree creation before any `worker-start` runs.
2. **Task files:** After approval, write per-task contract files (`docs/superpowers/tasks/<task-id>.md`) from each task section below and pass via `--spec-file`.
3. **Launch command (per task):**
   - Primary (`antigravity`, effort encoded in model ID—omit `--effort`):
     ```bash
     orca orchestration worker-start --spec-file <task-file> --task-title "<title>" \
       --agent antigravity --model <model-id> \
       --worktree new-child --base-branch usage-enhancements --name agent-<task-id> --json
     ```
   - Fallback / cross-model (`codex`):
     ```bash
     orca orchestration worker-start --spec-file <task-file> --task-title "<title>" \
       --agent codex --model <model-id> --effort <level> \
       --worktree new-child --base-branch usage-enhancements --name agent-<task-id> --json
     ```
   - Add `--deps '["<dep_task_id>", ...]'` for dependent waves (or wait for wave integration into `usage-enhancements` before starting the next wave so each wave branches off verified changes—recommended since `ModelChecks.swift`, `PixelIcon.swift`, and `StatusController.swift` are shared across waves).
   - Start all independent workers in the current wave, then **end turn** (Orca wakes coordinator; never poll).
4. **Verify & escalate:** On each `worker_done`, coordinator re-runs `swift run CoreChecks` in the worker worktree itself. Never trust worker output alone. On failure, retry one tier up with `--retry-of <dispatch_id>` and the higher tier's `--model` (and `--effort` for `codex`); never retry on the same setting.
5. **Merge & cleanup:** Human approves merging wave branches into `usage-enhancements`, pushing, and releasing/deleting worktrees.

## Global Constraints

- Native macOS AppKit only: no SwiftUI, no third-party packages, no helper daemons (`AGENTS.md`).
- Physical memory stays under 30 MB: verify with `footprint -p $(pgrep -x AIUsage) | grep phys_footprint`.
- No new polling timer beyond the single schedule timer in `Poller` (30 s, or 10 s when Refresh Every is 10 s or 30 s).
- All logic belongs in `Sources/AIUsageCore/` and is covered first in `Sources/CoreChecks/` (`swift run CoreChecks`).
- Never read or store credentials; CLI/MCP modes only read `~/Library/Application Support/AIUsage/usage.json`.
- Never edit version numbers by hand. Do not commit, push, merge, or delete worktrees unless the human asks.

## Review Focus

- **All cards hidden while cycling or in Most Urgent mode:** `Pin.cycleCandidates` returns `[]`; `Pin.cycle` / `Pin.mostUrgent` return `nil` without crashing (`% 0`) and `StatusController` falls back to `effective` / placeholder. (Pinned in Tasks 1 & 4.)
- **Stale `resetsAt` in provider response (provider hasn't rolled over yet):** `Schedule.nextReset(in:after: started)` only considers `resetsAt > started`, so a past reset timestamp from before the last poll never triggers an infinite refresh loop. (Pinned in Task 2.)
- **Missing/zero `durationSeconds` (Copilot) or clock skew (`now > resetsAt` / `now < resetsAt - duration`):** `WindowReport.elapsedFraction(resetsAt:durationSeconds:at:)` returns `nil` when `resetsAt == nil` or `durationSeconds <= 0`, and clamps to `0...1` on skew; `CardView` only draws a tick for `0 < elapsed < 1`. (Pinned in Tasks 3 & 6.)
- **Multi-account listing failure after prior success (e.g., `gh auth status` hiccup):** `ProviderRun.merging(previous:latest:)` detects a single config-level `.failure` (`id == configId`) replacing prior `configId:*` `.success` runs and preserves every prior account run with `staleError` set. (Pinned in Task 5.)
- **`--json` when app is not running or `usage.json` is corrupt:** `UsageSnapshot.cliJSON(load:now:)` returns a readable error message and `exitCode: 1` without touching AppKit or hanging. (Pinned in Task 6.)

---

### Task 1 (`t1`): Filter hidden cards out of `Cycle Menu Bar`

**Worker Routing:**
- Tier: **T1** (score `1`: A0/D0/S1/R0/V0) | Wave: **1** (parallel with `t2`, `t3`) | Depends on: none
- Primary: `--agent antigravity --model gemini-4-hydrogen-low` | Fallback: `--agent codex --model gpt-6.1-terra --effort low`
- Worktree: `--worktree new-child --base-branch usage-enhancements --name agent-t1`
- Acceptance: `swift run CoreChecks` → `ALL PASS`

**Files:**
- Modify: `Sources/AIUsageCore/PixelIcon.swift:181-191`
- Modify: `Sources/AIUsage/StatusController.swift:24-27,174,260`
- Test: `Sources/CoreChecks/ModelChecks.swift` (`pinChecks()`)

**Interfaces:**
- Produces:
  - `Pin.cycleCandidates(_ runs: [ProviderRun], order: [String], hidden: Set<String>) -> [ProviderRun]` — returns `CardOrder.sorted(runs, saved: order).filter { !hidden.contains($0.id) }`.

- [x] **Step 1: Write failing checks** in `Sources/CoreChecks/ModelChecks.swift` inside `pinChecks()`:
  - `cycleCandidatesExcludesHidden`: given runs `[claude, codex, copilot]`, `order: ["copilot", "claude", "codex"]`, `hidden: ["claude"]` → candidate ids are `["copilot", "codex"]`.
  - `cycleCandidatesAllHidden`: `hidden` contains all run ids → `cycleCandidates` is `[]`, and `Pin.cycle(0, in: [])` returns `nil`.

- [x] **Step 2: Run checks to verify failure**
  - Run: `swift run CoreChecks`
  - Expected: FAIL to compile (`cycleCandidates` undefined).

- [x] **Step 3: Implement `Pin.cycleCandidates`** in `Sources/AIUsageCore/PixelIcon.swift`
  - Add `public static func cycleCandidates(_ runs: [ProviderRun], order: [String], hidden: Set<String>) -> [ProviderRun]` returning `CardOrder.sorted(runs, saved: order).filter { !hidden.contains($0.id) }`.

- [x] **Step 4: Wire `StatusController.swift`**
  - Update `cycleRuns` (`StatusController.swift:174`) to `Pin.cycleCandidates(visibleRuns, order: cardOrder, hidden: hiddenCards)`.
  - Call `updateIcons()` after mutating `hiddenCards` in both the `hide` closure (`:24-27`) and `.showCard` handler (`:260`) so hiding/showing immediately updates a cycling icon.

- [x] **Step 5: Verify**
  - Run: `swift run CoreChecks`
  - Expected: `ALL PASS`.

---

### Task 2 (`t2`): Auto-refresh when a window's `resetsAt` passes

**Worker Routing:**
- Tier: **T2** (score `3`: A0/D1/S1/R1/V0) | Wave: **1** (parallel with `t1`, `t3`) | Depends on: none
- Primary: `--agent antigravity --model gemini-4-hydrogen-medium` | Fallback: `--agent codex --model gpt-6.1-terra --effort medium`
- Worktree: `--worktree new-child --base-branch usage-enhancements --name agent-t2`
- Acceptance: `swift run CoreChecks` → `ALL PASS`

**Files:**
- Modify: `Sources/AIUsageCore/MenuModel.swift:114-139` (`Schedule`)
- Modify: `Sources/AIUsage/Poller.swift:31-37,106-122`
- Test: `Sources/CoreChecks/ModelChecks.swift` (`menuChecks()`)

**Interfaces:**
- Produces:
  - `Schedule.nextReset(in runs: [ProviderRun], after started: Date) -> Date?` — earliest `window.resetsAt` strictly `> started` across all `.success` reports in `runs`.
  - `Schedule.nextRun(started: Date, interval: Int, failures: Int, resetsAt: Date? = nil) -> Date` — `started + delay(interval:failures:)`, or `resetsAt` when `resetsAt` is `> started` and earlier than the interval deadline (only when `failures == 0`, so failing providers still respect exponential backoff).
  - `Schedule.isDue(started: Date, interval: Int, failures: Int, now: Date, resetsAt: Date? = nil) -> Bool` — `nextRun(started:interval:failures:resetsAt:) <= now + 1`.

- [x] **Step 1: Write failing checks** in `Sources/CoreChecks/ModelChecks.swift` inside `menuChecks()`:
  - `nextResetPicksEarliestAfterStart`: windows resetting at `started - 10`, `started + 120`, `started + 60` → returns `started + 60`; ignores `.failure` and `nil` runs.
  - `nextResetIgnoresPastOrEqual`: only `resetsAt <= started` → returns `nil` (guards against infinite loop when provider hasn't rolled over).
  - `isDueAtPassedReset`: `interval: 300`, `started: t0`, `resetsAt: t0 + 90`, `now: t0 + 90` → `true`; `now: t0 + 80` → `false`.
  - `isDueBackoffOverridesReset`: `failures: 2`, `started: t0`, `resetsAt: t0 + 30`, `now: t0 + 40` → `false` (backoff delay 120 s still respected).
  - `nextRunPrefersEarlierReset`: `interval: 300`, `started: t0`, `resetsAt: t0 + 45` → `t0 + 45`; `resetsAt: t0 + 600` → `t0 + 300`.

- [x] **Step 2: Run checks to verify failure**
  - Run: `swift run CoreChecks`
  - Expected: FAIL to compile (`nextReset` / `nextRun` undefined).

- [x] **Step 3: Implement `Schedule.nextReset`, `Schedule.nextRun`, and extend `Schedule.isDue`** in `Sources/AIUsageCore/MenuModel.swift`
  - Keep `resetsAt: Date? = nil` default on `isDue` so existing callers/checks remain valid.

- [x] **Step 4: Wire `Poller.swift`**
  - Add private helper `resetDate(for configId: String, after started: Date) -> Date?` calling `Schedule.nextReset(in: runs.filter { $0.configId == configId }, after: started)`.
  - Use `Schedule.nextRun(...)` in `nextRefresh` (`Poller.swift:31-37`) and `Schedule.isDue(..., resetsAt: ...)` in `tick()` (`Poller.swift:106-122`).

- [x] **Step 5: Verify**
  - Run: `swift run CoreChecks`
  - Expected: `ALL PASS`.

---

### Task 3 (`t3`): Window pacing / elapsed tick on concentric rings

**Worker Routing:**
- Tier: **T2** (score `3`: A0/D1/S1/R0/V1) | Wave: **1** (parallel with `t1`, `t2`) | Depends on: none
- Primary: `--agent antigravity --model gemini-4-hydrogen-medium` | Fallback: `--agent codex --model gpt-6.1-terra --effort medium`
- Worktree: `--worktree new-child --base-branch usage-enhancements --name agent-t3`
- Acceptance: `swift run CoreChecks` → `ALL PASS` and `swift build` succeeds

**Files:**
- Modify: `Sources/AIUsageCore/Report.swift:62-83` (`WindowReport`)
- Modify: `Sources/AIUsageCore/CardModel.swift:3-9,167-222`
- Modify: `Sources/AIUsage/CardView.swift:428-466`
- Test: `Sources/CoreChecks/ModelChecks.swift` (`cardChecks()`)

**Interfaces:**
- Produces:
  - `WindowReport.elapsedFraction(resetsAt: Date?, durationSeconds: Int?, at now: Date) -> Double?` (static) and `WindowReport.elapsedFraction(at now: Date) -> Double?` (instance, delegates to static) — requires `let resetsAt`, `let duration = durationSeconds`, `duration > 0`; returns `min(max(1 - resetsAt.timeIntervalSince(now) / Double(duration), 0), 1)`.
  - `CardRing`: add `public let elapsed: Double?` with `public init(fraction: Double?, tone: Tone, elapsed: Double? = nil)` (default keeps existing tests/callers compiling).

- [x] **Step 1: Write failing checks** in `Sources/CoreChecks/ModelChecks.swift` inside `cardChecks()`:
  - `elapsedHalfway`: `durationSeconds: 18000` (5h), `resetsAt: now + 9000` → `abs(elapsed - 0.5) < 1e-9`.
  - `elapsedClampsBounds`: `resetsAt: now - 60` → `1.0`; `resetsAt: now + 20000` (beyond duration) → `0.0`.
  - `elapsedNilWhenMissingOrZero`: `durationSeconds: nil` → `nil`; `durationSeconds: 0` → `nil`; `resetsAt: nil` → `nil`.
  - `cardRingPopulatesElapsed`: `CardModel.card(...)` sets `charts[0].rings[0].elapsed` to `0.5` for a halfway session window and `nil` for a Copilot window without `durationSeconds`.

- [x] **Step 2: Run checks to verify failure**
  - Run: `swift run CoreChecks`
  - Expected: FAIL to compile (`elapsedFraction` / `CardRing.elapsed` undefined).

- [x] **Step 3: Implement `WindowReport.elapsedFraction(at:)` and `CardRing.elapsed`**
  - Add static + instance `elapsedFraction` in `Sources/AIUsageCore/Report.swift`.
  - Update `CardRing` and `CardModel.Env.ring(_:elapsed:)` in `Sources/AIUsageCore/CardModel.swift`; pass `window.elapsedFraction(at: now)` in `windowChart` and `meter.windows.first?.elapsedFraction(at: now)` in `mergedChart`.

- [x] **Step 4: Draw elapsed tick in `CardView.drawChart`** (`Sources/AIUsage/CardView.swift:428-466`)
  - For each ring where `let elapsed = chart.rings[index].elapsed`, `elapsed > 0`, `elapsed < 1`: draw a radial tick across the ring (`radius - thickness / 2 - 1` to `radius + thickness / 2 + 1`) at angle `-Double.pi / 2 + 2 * Double.pi * elapsed`, `lineWidth = 1.5`, color `NSColor.labelColor.withAlphaComponent(0.75)` (drawn after the ring arc so it stays visible whether ahead or behind pace).

- [x] **Step 5: Verify**
  - Run: `swift run CoreChecks && swift build`
  - Expected: `ALL PASS` and clean build.

---

### Task 4 (`t4`): "Most Urgent" menu bar mode

**Worker Routing:**
- Tier: **T2** (score `4`: A0/D1/S2/R1/V0) | Wave: **2** | Depends on: `t1`
- Primary: `--agent antigravity --model gemini-4-hydrogen-medium` | Fallback: `--agent codex --model gpt-6.1-terra --effort medium`
- Worktree: `--worktree new-child --base-branch usage-enhancements --name agent-t4` (after `t1` integrates into `usage-enhancements`)
- Acceptance: `swift run CoreChecks` → `ALL PASS` and `swift build` succeeds

**Files:**
- Modify: `Sources/AIUsageCore/PixelIcon.swift:179-227`
- Modify: `Sources/AIUsageCore/MenuModel.swift:5-80`
- Modify: `Sources/AIUsageCore/PanelModel.swift:5-74`
- Modify: `Sources/AIUsage/StatusController.swift:19-35,83-96,154-189,229-273`
- Test: `Sources/CoreChecks/ModelChecks.swift` (`pinChecks()`), `Sources/CoreChecks/PanelChecks.swift` (`panelChecks()`)

**Interfaces:**
- Consumes: `Pin.cycleCandidates(_:order:hidden:)` (`t1`).
- Produces:
  - `public enum MenuBarMode: String, Equatable { case pinned, cycle, urgent }` with `public static func resolve(stored: String?, legacyCycle: Bool) -> MenuBarMode` (migrates old `"cycleIcon": true` to `.cycle`, else `.pinned`).
  - `Pin.mostUrgent(in runs: [ProviderRun], order: [String], hidden: Set<String>) -> String?`:
    - Filters candidates via `Pin.cycleCandidates(runs, order: order, hidden: hidden)`.
    - Picks the candidate with highest `report.maxPercent` (ties broken by card order). For a grouped report (`CardModel.isGrouped(report)`), returns `Pin.key(run: run.id, meter: worstMeter.id)` where `worstMeter` has the highest `maxPercent`; otherwise `CardModel.titlePin(run)`.
    - If no candidate has a numeric `maxPercent`, prefers the first failing candidate (`CardModel.titlePin(run)`), else the first candidate, else `nil`.
  - `MenuEntry.mostUrgent(Bool)`, `PanelRow.Kind.mostUrgent` (title `"Most Urgent"`, checked when active).
  - `MenuModel.entries(..., cycling: Bool = false, mostUrgent: Bool = false, ...)`.

- [x] **Step 1: Write failing checks**
  - In `Sources/CoreChecks/ModelChecks.swift` (`pinChecks()`):
    - `mostUrgentPicksHighestPercent`: `claude` (40%), `codex` (85%), `copilot` (20%) → `"codex"`.
    - `mostUrgentPicksWorstGroupMeter`: `antigravity` with `Gemini` (10%) and `Claude/GPT` (92%) vs `claude` (50%) → `"antigravity|Claude/GPT"`.
    - `mostUrgentExcludesHidden`: `codex` (95%) is in `hidden` → picks `"claude"` (40%).
    - `mostUrgentTieBreaksByOrder`: `codex` (70%) before `claude` (70%) in `order` → `"codex"`.
    - `mostUrgentAllHidden`: returns `nil`.
    - `menuBarModeLegacyMigration`: `resolve(stored: nil, legacyCycle: true) == .cycle`, `resolve(stored: "urgent", legacyCycle: true) == .urgent`, `resolve(stored: "bogus", legacyCycle: false) == .pinned`.
  - In `Sources/CoreChecks/PanelChecks.swift` (`panelChecks()`):
    - `rowsMostUrgentChecked`: `PanelModel.rows([.mostUrgent(true)], expanded: [], refreshing: false)` produces one row `.mostUrgent`, title `"Most Urgent"`, `checked == true`.
    - `entriesHaveMostUrgent`: `MenuModel.entries(..., mostUrgent: true, ...)` contains `.mostUrgent(true)`.

- [x] **Step 2: Run checks to verify failure**
  - Run: `swift run CoreChecks`
  - Expected: FAIL to compile (`MenuBarMode` / `mostUrgent` undefined).

- [x] **Step 3: Implement `MenuBarMode`, `Pin.mostUrgent`, `MenuEntry.mostUrgent`, and `PanelRow.Kind.mostUrgent`**
  - Update `PixelIcon.swift`, `MenuModel.swift` (place `.mostUrgent(mostUrgent)` right after `.cycleIcon(cycling)`), and `PanelModel.swift`.

- [x] **Step 4: Wire `StatusController.swift`**
  - Replace `cycling: Bool` storage with `menuBarMode: MenuBarMode` backed by `"menuBarMode"` in `UserDefaults`, falling back via `MenuBarMode.resolve(stored: defaults.string(forKey: "menuBarMode"), legacyCycle: defaults.bool(forKey: "cycleIcon"))`.
  - In `updateIcons()`: switch on `menuBarMode`:
    - `.pinned`: `effective`
    - `.cycle`: `Pin.cycle(cycleStep, in: cycleRuns) ?? effective`
    - `.urgent`: `Pin.mostUrgent(in: visibleRuns, order: cardOrder, hidden: hiddenCards) ?? effective`
  - Clicking Pin on a card sets `menuBarMode = .pinned` (and stops cycle timer).
  - Clicking `.cycleIcon` toggles between `.cycle` and `.pinned`; clicking `.mostUrgent` toggles between `.urgent` and `.pinned`. Always call `updateCycleTimer()` and `updateIcons()`.

- [x] **Step 5: Verify**
  - Run: `swift run CoreChecks && swift build`
  - Expected: `ALL PASS` and clean build.

---

### Task 5 (`t5`): Retain last-known report on transient error + per-card retry

**Worker Routing:**
- Tier: **T3** (score `7`: A1/D2/S2/R1/V1) | Wave: **3** | Depends on: `t2`, `t3`, `t4`
- Primary: `--agent antigravity --model gemini-4-hydrogen-high` | Fallback: `--agent codex --model gpt-6.1-sol --effort medium`
- Worktree: `--worktree new-child --base-branch usage-enhancements --name agent-t5`
- Acceptance: `swift run CoreChecks` → `ALL PASS`, `swift build` succeeds, and `scripts/make-panel.swift` compiles cleanly

**Files:**
- Modify: `Sources/AIUsageCore/PixelIcon.swift:149-177` (`ProviderRun`)
- Modify: `Sources/AIUsageCore/CardModel.swift:48-70,122-156` (`Card`, `CardModel`)
- Modify: `Sources/AIUsageCore/UsageSnapshot.swift:93-118`
- Modify: `Sources/AIUsage/Poller.swift:80-137`
- Modify: `Sources/AIUsage/CardView.swift:25-127,251-285,482-512,581-602`
- Modify: `Sources/AIUsage/PanelController.swift:12-18` (and card construction site)
- Modify: `Sources/AIUsage/StatusController.swift:19-35`
- Test: `Sources/CoreChecks/ModelChecks.swift` (`cardChecks()`), `Sources/CoreChecks/UsageChecks.swift` (`usageChecks()`)

**Interfaces:**
- Produces:
  - `ProviderRun.staleError: String?` (default `nil` in both initializers).
  - `ProviderRun.merging(previous: [ProviderRun], latest: [ProviderRun]) -> [ProviderRun]`:
    - If `latest` is a single `.failure(err)` whose `id == configId` and `previous` has at least one `.success` run with `configId == latest[0].configId`, return those previous `.success` runs with `staleError = err.message` (covers multi-account listing failure).
    - Otherwise, for each run in `latest`: if `.failure(err)` and matching `previous` run (same `id`) has `.success(report)`, emit `ProviderRun` keeping `.success(report)` (and previous `iconLabel`/`name`) with `staleError = err.message`. If `.success`, `staleError = nil`. If `.failure` with no prior `.success`, keep `.failure(err)` with `staleError = nil`.
  - `Card.configId: String` (default `id` in `Card.init` so existing tests stay untouched).
  - `Poller.refresh(configId: String)` — resets `states[configId]?.started = .distantPast`, `states[configId]?.failures = 0`, calls `tick()`.
  - `PanelActions.retry: (String) -> Void`; `CardView.init(..., onRetry: @escaping (String) -> Void = { _ in })` (default keeps `scripts/make-panel.swift` compiling).

- [x] **Step 1: Write failing checks**
  - In `Sources/CoreChecks/ModelChecks.swift` (`cardChecks()`):
    - `mergeKeepsLastGoodOnFailure`: previous `.success(report)` + latest `.failure("timed out")` → merged has `.success(report)` and `staleError == "timed out"`.
    - `mergeClearsStaleOnSuccess`: previous `.success` with `staleError` + latest `.success` → `staleError == nil`.
    - `mergeKeepsFailureWithoutPriorSuccess`: previous `nil` or `.failure` + latest `.failure("no cli")` → `.failure("no cli")`, `staleError == nil`.
    - `mergePreservesMultiAccountOnListFailure`: previous `[copilot:alice (.success), copilot:bob (.success)]` + latest `[copilot (.failure("gh failed"))]` → both account runs preserved with `staleError == "gh failed"`.
    - `cardShowsStaleNotice`: `CardModel.card` on `.success` with `staleError: "timed out"` keeps charts AND appends `CardNotice(text: "Stale: timed out", tone: .orange)` to `card.notices`; `card.configId == run.configId`.
  - In `Sources/CoreChecks/UsageChecks.swift` (`usageChecks()`):
    - `usageStaleErrorKeepsNumbers`: `.success` run with `staleError: "timed out"` produces `status == "ok"`, valid `remainingPercent`, and `message == "Stale: timed out"`.

- [x] **Step 2: Run checks to verify failure**
  - Run: `swift run CoreChecks`
  - Expected: FAIL to compile (`staleError` / `merging` / `configId` undefined).

- [x] **Step 3: Implement Core changes**
  - Update `ProviderRun` + `ProviderRun.merging(previous:latest:)` in `Sources/AIUsageCore/PixelIcon.swift`.
  - Update `Card` + `CardModel.card` in `Sources/AIUsageCore/CardModel.swift` (append `CardNotice(text: "Stale: \(err)", tone: .orange)` when `run.staleError` is non-nil).
  - Update `UsageSnapshot.provider(from:)` in `Sources/AIUsageCore/UsageSnapshot.swift` to pass `message: run.staleError.map { "Stale: \($0)" }` on `.success`.

- [x] **Step 4: Wire `Poller`, `CardView`, `PanelController`, `StatusController`**
  - In `Poller.finish(_:_:)` (`Poller.swift:125-136`): compute `failed` from raw `results` BEFORE merging (preserves exponential backoff!), then replace `config.id` runs with `ProviderRun.merging(previous: previous, latest: results)`.
  - Add `Poller.refresh(configId: String)`.
  - In `CardView.swift`: add `.retry` case to `PinTarget.Kind` (SF Symbol `"arrow.clockwise"`, accessibility label `"Refresh \(target.label)"`), draw hover retry button immediately left of the move/hide buttons, pass `card.configId` as its key, and add `retryWidth` (`box + 4`) to `CardView.width(for:leading:)`.
  - Wire `PanelActions.retry` in `PanelController.swift` and `StatusController.swift` (`{ [weak self] id in self?.poller.refresh(configId: id) }`).

- [x] **Step 5: Verify**
  - Run: `swift run CoreChecks && swift build`
  - Expected: `ALL PASS` and clean build.

---

### Task 6 (`t6`): `--json` CLI flag, richer `UsageSnapshot` fields, and `README.md`

**Worker Routing:**
- Tier: **T2** (score `3`: A0/D1/S1/R1/V0) | Wave: **4** | Depends on: `t3`, `t5`
- Primary: `--agent antigravity --model gemini-4-hydrogen-medium` | Fallback: `--agent codex --model gpt-6.1-terra --effort medium`
- Worktree: `--worktree new-child --base-branch usage-enhancements --name agent-t6`
- Acceptance: `swift run CoreChecks` → `ALL PASS` and `swift build -c release` succeeds

**Files:**
- Modify: `Sources/AIUsageCore/SnapshotFile.swift` (shared `isNotFound`)
- Modify: `Sources/AIUsageCore/McpServer.swift` (use `SnapshotFile.isNotFound`)
- Modify: `Sources/AIUsageCore/UsageSnapshot.swift:1-127`
- Modify: `Sources/AIUsage/main.swift:8-22`
- Modify: `README.md`
- Test: `Sources/CoreChecks/UsageChecks.swift` (`usageChecks()`, `mcpChecks()`)

**Interfaces:**
- Consumes: `WindowReport.elapsedFraction(resetsAt:durationSeconds:at:)` (`t3`).
- Produces:
  - `public struct UsageCount: Codable, Equatable { public let used: Double; public let limit: Double; public let unit: String? }`
  - `UsageWindow`: add `public let durationSeconds: Int?` (default `nil`) and `public let count: UsageCount?` (default `nil`).
  - `UsageSnapshot.view(now:)` window dict adds:
    - `"durationSeconds": Int` when present.
    - `"elapsedPercent": Double` (computed live via `WindowReport.elapsedFraction(resetsAt: w.resetsAt, durationSeconds: w.durationSeconds, at: now)` rounded to 1 decimal place: `(fraction * 1000).rounded() / 10`) when non-nil.
    - `"count": ["used": Double, "limit": Double, "unit": String?]` when present.
  - `UsageSnapshot.cliJSON(load: () throws -> Data, now: Date = Date()) -> (output: String, exitCode: Int32)` — returns compact single-line JSON string and `0` on success; on missing file or decode error returns `"{\"error\":\"...\"}"` and `1`.

- [x] **Step 1: Write failing checks** in `Sources/CoreChecks/UsageChecks.swift`:
  - `usageWindowMetadata`: window with `durationSeconds: 18000`, `resetsAt: now + 9000`, `count: WindowCount(used: 40, limit: 100, unit: "credits")` → round-trips through `encoded()`/`decode(_:)`, and `view(now:)` includes `"durationSeconds": 18000`, `"elapsedPercent": 50.0`, and `"count": {"used": 40, "limit": 100, "unit": "credits"}`.
  - `usageWindowElapsedTracksViewNow`: same snapshot viewed at `now + 4500` has `"elapsedPercent": 75.0` (computed live against `view(now:)`, not frozen at `generatedAt`).
  - `usageWindowNoDurationOmitsElapsed`: window without `durationSeconds` omits `"durationSeconds"` and `"elapsedPercent"` keys.
  - `cliJSONSuccess`: `UsageSnapshot.cliJSON(load: { encoded }, now: now)` returns `exitCode == 0` and valid JSON containing `"providers"`.
  - `cliJSONMissingFile`: loader throwing `ENOENT` returns `exitCode == 1` and JSON containing `"error"` and `"not running"`.
  - `cliJSONCorruptFile`: loader returning `Data("{".utf8)` returns `exitCode == 1` and JSON containing `"error"`.

- [x] **Step 2: Run checks to verify failure**
  - Run: `swift run CoreChecks`
  - Expected: FAIL to compile (`UsageCount` / `cliJSON` undefined).

- [x] **Step 3: Implement `UsageCount`, `UsageWindow` extensions, `view(now:)`, and `UsageSnapshot.cliJSON`** in `Sources/AIUsageCore/UsageSnapshot.swift`
  - Add `static func isNotFound(_ error: Error) -> Bool` on `SnapshotFile` and use it in both `McpServer` and `UsageSnapshot.cliJSON` for the `"AIUsage app is not running or has no snapshot yet"` message.

- [x] **Step 4: Wire `--json` in `Sources/AIUsage/main.swift`**
  - Before `NSApplication.shared`, add:
    ```swift
    if CommandLine.arguments.contains("--json") {
        let result = UsageSnapshot.cliJSON(load: { try Data(contentsOf: SnapshotFile.url) }, now: Date())
        print(result.output)
        fflush(stdout)
        exit(result.exitCode)
    }
    ```

- [x] **Step 5: Update `README.md`**
  - Document `--json` usage, new `windows` fields (`durationSeconds`, `elapsedPercent`, `count`), **Most Urgent** menu bar mode, ring elapsed tick, automatic refresh on reset, stale-on-error banner, and per-card retry button.

- [x] **Step 6: Verify**
  - Run: `swift run CoreChecks && swift build -c release`
  - Expected: `ALL PASS` and clean release build.

---

### Task 7 (`t7`): Whole-branch cross-model review & smoke verification

**Worker Routing:**
- Tier: **T3** (reviewer floor `>= t5`) | Wave: **5** | Depends on: `t6`
- Primary: `--agent codex --model gpt-6.1-sol --effort medium` (cross-model second pair of eyes) | Fallback: `--agent antigravity --model gemini-4-hydrogen-high`
- Worktree: `--worktree current` (read-only review on `usage-enhancements`; reviewer reports verdict, never edits code)
- Acceptance: JSON review verdict `"pass"` + coordinator verifies `CoreChecks`, `make-panel.swift` compile, `--json` smoke test, and `footprint < 30 MB`

- [x] **Step 1: Dispatch read-only reviewer** against `git diff main..HEAD` checking all 5 **Review Focus** edge cases, `AGENTS.md` constraints (AppKit-only, no extra timers, no credentials), and `CardView` layout/a11y regressions.
- [x] **Step 2: Coordinator final verification**
  - Run `swift run CoreChecks` → `ALL PASS`.
  - Run `swift build -c release` → succeeds cleanly.
  - Verify `scripts/make-panel.swift` compiles cleanly per README build snippet (without overwriting `docs/images/panel.png` unless requested).
  - Smoke-test `.build/release/AIUsage --json` against running app → exits `0` with `providers`, `durationSeconds`, `elapsedPercent`, `count`.
  - Run `footprint -p $(pgrep -x AIUsage) | grep phys_footprint` → under 30 MB.
