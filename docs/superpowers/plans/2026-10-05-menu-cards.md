# Menu Panel Cards Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the text menu panel with one card per provider (concentric radial chart on the left, reset date and `in 2 minutes` on the right), use a pin icon instead of stars, and remove cycle mode.

**Architecture:** All logic (ring order, rows, relative time, default pin) is pure Swift in `AIUsageCore`, tested by `CoreChecks`. `MenuModel.entries` returns a `.card(Card)` entry per provider run. In the app target, a `CardView: NSView` draws the model with Core Graphics and is set as `NSMenuItem.view`. The icon always shows one pin; the 5 s cycle timer goes away.

**Tech Stack:** Swift 6 tools / Swift 5 mode, AppKit only, SF Symbols `pin` / `pin.fill`, no packages.

**Spec:** `docs/specs/menubar/menubar-panel.md`, `docs/specs/menubar/menubar-icon.md`

## Global Constraints

- AppKit only; no SwiftUI, no third-party packages, no new timers (only the 30 s schedule timer remains).
- Physical footprint stays under 30 MB: `footprint -p $(pgrep -x AIUsage) | grep phys_footprint` after the AppKit task.
- Logic goes in `Sources/AIUsageCore/`; write the `CoreChecks` check first (`swift run CoreChecks` must end `ALL PASS`).
- Ring tone: green below 70%, orange from 70%, red from 90% (`StatusIcons.severity`). Max 4 rings per chart, max 4 charts per card.
- Ring order: session (5 h) outermost, weekly (7 d) next, other windows in file order. Merged chart (every meter has one unlabeled window): Premium Interactions first, then file order.
- Reset line: `<MMM d, yyyy h:mm a> · in 2 minutes`; past reset shows `resetting…`; no reset time shows no line.
- Never read or store credentials. Do not edit version numbers. No commits or pushes unless the human asks (this overrides any commit step you would normally add).

## Review Focus

- A ring window with `usedPercent == nil` draws the track only and its row shows `?`.
- `usedPercent` above 100 or below 0 clamps the ring fraction to 0...1; the row text keeps the real number.
- Reset exactly now, in the past, and under 1 minute (`in 1 second`, `in 45 seconds`).
- More than 4 windows in one meter, or more than 4 groups: extras are text rows without rings, nothing is dropped.
- Stored pin points to a disabled, removed, or failed provider, a missing limit, or the legacy provider-level pin `antigravity`: icon and panel agree on the normalized pin.
- No provider on: icon shows gray `AI` over `--`, panel shows `All providers are off`.
- Pinned provider still loading: icon shows `--` in gray with the provider's label, not the placeholder.

---

### Task 1: Relative time text

**Files:**
- Create: `Sources/AIUsageCore/RelativeTime.swift`
- Test: `Sources/CoreChecks/ModelChecks.swift` (new `cardChecks()` function, registered in `main.swift` after `toggleChecks()`)

**Interfaces:**
- Produces: `public enum RelativeTime { public static func text(until date: Date, now: Date) -> String? }`

- [ ] **Step 1: Write the failing check `relativeTime` in `cardChecks()`.** With `now = Date(timeIntervalSince1970: 1_000_000)`: +45 s → `"in 45 seconds"`, +1 s → `"in 1 second"`, +120 s → `"in 2 minutes"`, +119 s → `"in 1 minute"`, +3 h → `"in 3 hours"`, +4 d → `"in 4 days"`, +86 399 s → `"in 23 hours"`, now → `nil`, −5 s → `nil`.
- [ ] **Step 2: Run `swift run CoreChecks`.** Expected: build error, `RelativeTime` not defined.
- [ ] **Step 3: Implement `RelativeTime.text(until:now:)`.** Floor each unit (seconds under 60, minutes under 3600, hours under 86 400, else days); singular for 1; `nil` when `date <= now`.
- [ ] **Step 4: Run `swift run CoreChecks`.** Expected: `ALL PASS`.

### Task 2: Card model

**Files:**
- Create: `Sources/AIUsageCore/CardModel.swift`
- Test: `Sources/CoreChecks/ModelChecks.swift` (`cardChecks()`)

**Interfaces:**
- Consumes: `RelativeTime.text(until:now:)`; `Report.headlinePercent`, `MeterReport`, `WindowReport`; `StatusIcons.severity`, `StatusIcons.percentText`; `Tone`; `Pin.key(run:meter:)`.
- Produces:
  - `public struct CardRing: Equatable { public let fraction: Double?; public let tone: Tone }` (fraction clamped to 0...1, `nil` when no percent)
  - `public struct CardRow: Equatable { public let label: String; public let percentText: String; public let tone: Tone; public let reset: String?; public let ringIndex: Int?; public let pin: String?; public let pinned: Bool }`
  - `public struct CardChart: Equatable { public let title: String?; public let pin: String?; public let pinned: Bool; public let center: String; public let rings: [CardRing]; public let rows: [CardRow] }`
  - `public struct CardNotice: Equatable { public let text: String; public let tone: Tone }`
  - `public struct Card: Equatable { public let id: String; public let title: String; public let pin: String; public let pinned: Bool; public let message: CardNotice?; public let notices: [CardNotice]; public let charts: [CardChart]; public let overflow: [CardRow] }` (`message` is the loading or error text; `notices` are the access and plan-limit lines)
  - `public enum CardModel { public static func card(_ run: ProviderRun, pinned: String?, now: Date, formatReset: (Date) -> String) -> Card; public static func isGrouped(_ report: Report) -> Bool; public static func titlePin(_ run: ProviderRun) -> String }`

- [ ] **Step 1: Write failing checks in `cardChecks()`.** Build runs from the existing `pinChecks()` fixtures (Antigravity with `Gemini Models` [Weekly 7, Five Hour 3 with `durationSeconds` 18 000] and `Claude and GPT models` [Weekly 95]; Claude-like single meter `plan` with Session 10 and Weekly 80; Copilot-like meters `chat` 0, `premium_interactions` 79, `completions` 5 with one unlabeled window each). Use a fixed `now` and a `formatReset` that returns `"D"`. Assert:
  - Claude card: 1 chart, no chart title, rings = 2 (Session ring first with fraction 0.10 and tone `.green`, Weekly 0.80 `.orange`), `center == "10%"`, rows' `ringIndex` `[0, 1]`, `pin == "claude"`, rows have `pin == nil`.
  - Antigravity card: 2 charts titled `Gemini Models` (rings: Five Hour 0.03 first, Weekly 0.07; center `3%`; `pin == "antigravity|Gemini Models"`) and `Claude and GPT models` (1 ring 0.95 `.red`, center `95%`); card `pin == "antigravity|Gemini Models"` and `pinned == true` only when `pinned` input equals that key, false for `"antigravity|Claude and GPT models"`.
  - Copilot-like card: 1 chart, rings ordered Premium, Chat, Completions; each row has `pin == "copilot:a|<meter id>"`; card `pin == "copilot:a"`, `pinned == false` when a limit is pinned.
  - Reset line: window with `resetsAt = now + 120` and `formatReset → "D"` gives `"D · in 2 minutes"`; `resetsAt = now − 1` gives `"resetting…"`; no `resetsAt` gives `nil`.
  - Overflow: a meter with 5 windows gives 4 rings and one row with `ringIndex == nil`; a report with 5 groups gives 4 charts and `overflow.count == 1` (label = group name, percent = group headline).
  - Failure run: `message == CardNotice(text: "Error: boom", tone: .red)`, `charts.isEmpty`; loading run: message `Loading…` secondary; `report.access == false` and `available == false` add notices as before (`Usage limit reached` red, `Plan limits don't apply to this account` secondary); empty meters: `No usage data` secondary. Title is `Name (plan)` or `Name`.
- [ ] **Step 2: Run `swift run CoreChecks`.** Expected: build error, `CardModel` not defined.
- [ ] **Step 3: Implement the types and `CardModel` above in `CardModel.swift`.** Grouped means more than one meter and not every meter has a single unlabeled window. Charts: not grouped and one meter gives one chart with a ring per window; not grouped with several meters (Copilot) gives one merged chart with a ring per meter (percent = `MeterReport.maxPercent`); grouped gives one chart per meter, up to 4. `center` = `Report(meters: chartMeters).headlinePercent` as `StatusIcons.percentText`, `?` when nil. Ring tone from `StatusIcons.severity` of the ring's percent (`.secondary` when nil). `titlePin` is the first group's `Pin.key` when grouped, else `run.id`. Row pins: group header pin (`CardChart.pin`) when grouped, row pin on merged-chart rows, none otherwise.
- [ ] **Step 4: Run `swift run CoreChecks`.** Expected: `ALL PASS`.

### Task 3: Effective pin, single icon, menu entries

**Files:**
- Modify: `Sources/AIUsageCore/PixelIcon.swift` (`Pin`, `StatusIcons.icons`)
- Modify: `Sources/AIUsageCore/MenuModel.swift`
- Modify: `Sources/CoreChecks/ModelChecks.swift` (`iconChecks`, `menuChecks`, `pinChecks`, `toggleChecks`), `Sources/CoreChecks/Check.swift` (`liveRun`)
- Test: same files

**Interfaces:**
- Consumes: `CardModel.card`, `CardModel.isGrouped`, `CardModel.titlePin`.
- Produces:
  - `Pin.effective(_ pin: String?, in runs: [ProviderRun]) -> String?` returns a valid pin: the resolved pin kept as is if its run (and limit, if any) exists; a provider-level pin on a grouped provider becomes its first group key; a missing limit falls back to its run id (then normalized as above); otherwise the first run's `titlePin`; `nil` only when `runs` is empty.
  - `StatusIcons.icon(runs: [ProviderRun], pinned: String?) -> IconSpec` replaces `icons(...)`: the effective pin's icon; `StatusIcons.placeholder` when `runs` is empty; a loading pinned provider gives `IconSpec(top: run.iconLabel, bottom: "--", gray, ...)`.
  - `MenuEntry.card(Card)` replaces `.provider`, `.meter`, `.cycleAll`; `.detail` stays for config errors and `All providers are off`. `MenuModel.entries(...)` keeps its parameters and gains `now: Date`.

- [ ] **Step 1: Rewrite the existing checks first.** In `iconChecks`, replace `unpinnedCycles`, `cycleIncludesErrors`, `loadingLeftOut`, `nothingLoadedPlaceholder` with: `Pin.effective(nil, in: [codex, copilot]) == "codex"`; legacy `"antigravity"` on the Antigravity fixture gives `"antigravity|Gemini Models"`; `"antigravity|gone"` gives the same; `"nope"` gives the first run; `[]` gives `nil`; `icon(runs: [], pinned: nil) == StatusIcons.placeholder`; failed pinned run shows `ERR`; loading pinned run shows `--`. In `menuChecks`/`pinChecks`/`toggleChecks`, replace star-title and `.cycleAll` assertions with: entries contain exactly one `.card` per visible run, no `.cycleAll` case exists, and the last items are unchanged. Keep `meterPinIcon`, `meterPinAccessibility`, `meterLabel` and the headline checks passing (they use `Pin.key` and `StatusIcons.label`).
- [ ] **Step 2: Run `swift run CoreChecks`.** Expected: build errors on the removed cases and the missing functions.
- [ ] **Step 3: Implement `Pin.effective` and `StatusIcons.icon(runs:pinned:)`; delete `StatusIcons.icons`, `MenuEntry.provider/.meter/.cycleAll`, `MenuModel.star`, `providerEntries`, `windowEntry`.** `entries` builds `.card(CardModel.card(run, pinned: effectivePin, now: now, formatReset: formatReset))` per run. Update `liveRun()` in `Check.swift` to print cards and the single icon.
- [ ] **Step 4: Run `swift run CoreChecks`.** Expected: `ALL PASS`.

### Task 4: AppKit cards and no cycle

**Files:**
- Create: `Sources/AIUsage/CardView.swift`
- Modify: `Sources/AIUsage/StatusController.swift`

**Interfaces:**
- Consumes: `Card`, `CardChart`, `CardRing`, `CardRow`, `CardNotice`, `Tone`, `MenuEntry.card`, `Pin.effective`, `StatusIcons.icon`.
- Produces: `final class CardView: NSView { init(card: Card, onPin: @escaping (String) -> Void) }` (fixed 300 pt wide; height from content).

- [ ] **Step 1: Implement `CardView` drawing and hit testing.** Title row: SF Symbol `pin` (`.secondaryLabelColor`) or `pin.fill` (`.controlAccentColor`), bold title when pinned. Each chart: rings drawn with `NSBezierPath` arcs from 12 o'clock clockwise over a track in `NSColor.separatorColor`, outermost ring first, center text from `CardChart.center`; to the right a dot in the ring's tone color, `label`, `percentText`, and the `reset` line in `.secondaryLabelColor` under it. Chart header (grouped) shows `title` and its own pin. Message and notices as text under the title. `mouseUp` hit-tests pin rects and calls `onPin(key)` then `enclosingMenuItem?.menu?.cancelTracking()`. Use system colors only; set `setAccessibilityLabel` summarizing the card and make each pin an accessibility button labeled `Pin <name>` / `Unpin <name>`.
- [ ] **Step 2: Wire `StatusController`.** Delete `cycleIndex`, `cycleTimer`, the 5 s timer, `cycleAll`, and the `.cycleAll`/`.provider`/`.meter` cases of `item(for:)`; `.card` becomes an `NSMenuItem` with `view = CardView(...)`. `updateIcons()` computes `let effective = Pin.effective(pinned, in: visibleRuns)`, writes it back to `pinned` when different and non-nil, then sets one image from `StatusIcons.icon(runs:pinned:)`. Pass `now: Date()` into `MenuModel.entries`. `pin(_:)` accepts the key from `onPin`. Update `pinned` doc comment (no longer `nil` cycles).
- [ ] **Step 3: Build and run.** `scripts/build-app.sh && open ~/Applications/"AI Usage.app"`; open the menu and compare with the mockups in the spec for Claude, Copilot, and Antigravity (light and dark appearance). Verify click on a pin moves the icon and the stored pin, and no 5 s change occurs.
- [ ] **Step 4: Check footprint.** `footprint -p $(pgrep -x AIUsage) | grep phys_footprint` is under 30 MB.
- [ ] **Step 5: Run `swift run CoreChecks` and `swift run CoreChecks --live`.** Expected: `ALL PASS` and the live output lists cards for the shipped providers.

### Task 5: Docs

**Files:**
- Modify: `docs/specs/menubar/menubar-panel.md` (status line), `docs/specs/tech-stack.md` (timers), `README.md` (menu bar bullets: remove cycle mode and star text, describe cards and pin icon, default pin), `docs/specs/menubar/menubar-icon.md` only if the built behavior differs.

- [ ] **Step 1: Edit the docs.** Set the panel spec status to implemented; tech-stack timers line becomes `30 s schedule timer only`; README bullets match the spec.
- [ ] **Step 2: Re-read each edited doc against the built app.** Expected: no mention of `☆`, `★`, `Cycle All Providers`, or the 5 s cycle remains (`grep -rn "Cycle All\|5 seconds\|cycle" README.md docs/specs` shows none outside the "no cycle mode" statements).
