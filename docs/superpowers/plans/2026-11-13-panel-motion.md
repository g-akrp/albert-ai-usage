# Own Panel With Motion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `NSMenu` panel with our own `NSPanel` that stays open on clicks and animates every change.

**Architecture:** `StatusController` keeps the icon, settings and actions, and hands a `PanelContent` value to a new `PanelController`. `PanelController` owns the window and keeps one `CardView` per card id and one `RowView` per row, moving them with `NSAnimationContext`. Pure logic (row list, stacking, placement, tweening, keyboard highlight) lives in `AIUsageCore` and is checked by `swift run CoreChecks`.

**Tech Stack:** Swift 5 mode, AppKit, macOS 13+, no packages.

**Spec:** `docs/superpowers/specs/2026-11-13-panel-motion-design.md`

## Global Constraints

- Swift, AppKit only. No SwiftUI, no packages, no helper processes.
- Physical footprint under 30 MB: `footprint -p $(pgrep -x AIUsage) | grep phys_footprint`.
- No timer runs while nothing animates. No polling beyond the existing 30 s schedule timer. Event monitors exist only while the panel is open.
- Logic goes in `Sources/AIUsageCore/`, check written first in `Sources/CoreChecks/`. Run with `swift run CoreChecks`; the last line must be `ALL PASS`.
- Every duration is 0 when `NSWorkspace.shared.accessibilityDisplayShouldReduceMotion` is true. Use one helper, `Motion.duration(_:)`, everywhere.
- Card layout, colors, fonts and `UserDefaults` keys do not change.
- `CardView.init(card:width:leading:collapsed:canMoveUp:canMoveDown:onPin:onHide:onMove:onToggle:)` keeps its signature: `scripts/make-panel.swift` calls it, and a card that was never animated must draw its final state.
- Do not edit version numbers. Do not commit: `AGENTS.md` says commit only when the human asks. (This plan has no commit steps for that reason.)

## Review Focus

1. A second click before an animation ends (collapse twice within 0.22 s, move up then down): the panel ends in the state the settings say, starting each animation from what is on screen. Pinned by `tweenStartsFromScreen` (Task 1) and a manual step (Task 6).
2. A refresh result arrives while the panel is closed or fading out: no crash; the next open shows current values. Manual step in Task 6.
3. Old and new card differ in shape (a ring or a chart added or removed by a refresh): rings without an old value sweep from 0; nothing indexes out of range. Pinned by `tweenRingAdded`, `tweenChartRemoved` (Task 1).
4. Icon near the right screen edge, or on a second display: the panel is fully on the screen that holds the icon. Pinned by `originClampsRight`, `originOtherScreen` (Task 1).
5. More cards than fit the screen: the panel is as tall as the screen allows and scrolls; Quit is reachable. Pinned by `heightCapped` (Task 1) and a manual step (Task 6).

---

### Task 1: Pure panel logic in Core

**Files:**
- Create: `Sources/AIUsageCore/PanelModel.swift`
- Create: `Sources/CoreChecks/PanelChecks.swift`
- Modify: `Sources/CoreChecks/main.swift` (call `panelChecks()` after `cardChecks()`)

**Interfaces:**
- Consumes: `MenuEntry`, `Card`, `Tone` (existing).
- Produces:

```swift
public struct PanelRow: Equatable {
    public enum Kind: Equatable {
        case separator, detail, refresh, updated, providers, provider(id: String), hiddenCards,
             showCard(id: String), launchAtLogin, openProvidersFolder, version, credit(before: String, after: String), quit
    }
    public let kind: Kind
    public let title: String
    public let tone: Tone          // .normal except for .detail
    public let enabled: Bool       // false: gray, no hover, skipped by the keyboard
    public let checked: Bool?      // nil: no check mark column
    public let shortcut: String?   // "⌘R", "⌘Q"
    public let indented: Bool      // a row inside an open Providers or Hidden Cards section
    public let expanded: Bool?     // nil: no chevron
    public let spinning: Bool      // Refresh Now while a provider runs
}

public enum PanelModel {
    public static let providersSection = "providers", hiddenSection = "hiddenCards"
    public static func cards(_ entries: [MenuEntry]) -> [Card]
    public static func rows(_ entries: [MenuEntry], expanded: Set<String>, refreshing: Bool) -> [PanelRow]
    /// The row Up (`step` -1) or Down (1) lands on. `nil` current: first enabled for 1, last for -1. No wrap.
    public static func highlight(from current: Int?, step: Int, enabled: [Bool]) -> Int?
}

public enum PanelLayout {
    /// Top y of each view stacked from 0, and the total height.
    public static func stack(_ heights: [CGFloat]) -> (tops: [CGFloat], total: CGFloat)
    /// Panel height: the content, at most `available`.
    public static func height(content: CGFloat, available: CGFloat) -> CGFloat
    /// Panel origin (bottom left, screen coordinates): top edge `gap` under `anchor`, left edges aligned,
    /// moved left to stay `margin` inside `screen`'s right edge and right to stay inside its left edge.
    public static func origin(anchor: CGRect, size: CGSize, screen: CGRect, gap: CGFloat = 4, margin: CGFloat = 8) -> CGPoint
}

public enum Tween {
    /// `progress` is clamped to 0...1.
    public static func value(_ from: Double, _ to: Double, _ progress: Double) -> Double
    /// One array per chart of `new`, one fraction per ring. A ring with no counterpart in `old`
    /// (by chart index and ring index) starts at 0. A ring whose new fraction is nil stays nil.
    public static func fractions(from old: Card?, to new: Card, progress: Double) -> [[Double?]]
    /// Counts between two texts that are exactly digits plus "%" ("3%" to "14%"); any other text returns `to`.
    public static func percent(from: String?, to: String, progress: Double) -> String
}
```

`rows` rules: `.card` entries are skipped. `.providers` gives one row (`expanded` set from `expanded.contains(providersSection)`), followed, when expanded, by one `.provider` row per toggle (`checked` = `enabled`, `indented` true). `.hiddenCards` the same with `.showCard` rows titled `Show <name>`. `.refresh`: title `Refresh Now`, shortcut `⌘R`, `spinning` = `refreshing`. `.updated`, `.version`, `.credit`: `enabled` false; credit's title is `"\(before) \(after)"` (the view draws the heart). `.launchAtLogin(on)`: `checked` = `on`. `.openProvidersFolder`: `Open Providers Folder…`. `.quit`: `Quit AI Usage`, `⌘Q`. `.detail(text, tone)`: `enabled` false, `tone` kept. `.separator`: `enabled` false.

- [ ] **Step 1: Write the failing checks** in `Sources/CoreChecks/PanelChecks.swift` as `func panelChecks()`, in the style of `ModelChecks.swift` (`check("name", condition)`). Build entries with `MenuModel.entries(...)` the way `menuChecks()` does, or by hand.

```swift
// rows
let toggles = [ProviderToggle(id: "a", name: "A", enabled: true), ProviderToggle(id: "b", name: "B", enabled: false)]
let entries: [MenuEntry] = [.refresh, .updated("Updated 1:00 PM"), .separator, .providers(toggles),
                            .hiddenCards([HiddenCard(id: "h", name: "Codex")]), .launchAtLogin(true), .openProvidersFolder, .quit]
let closed = PanelModel.rows(entries, expanded: [], refreshing: false)
check("rowsClosedCount", closed.count == 8)
check("rowsRefreshShortcut", closed[0].shortcut == "⌘R" && !closed[0].spinning)
check("rowsUpdatedDisabled", !closed[1].enabled)
check("rowsProvidersChevron", closed[3].expanded == false)
check("rowsLaunchChecked", closed[5].checked == true)
let open = PanelModel.rows(entries, expanded: [PanelModel.providersSection, PanelModel.hiddenSection], refreshing: true)
check("rowsOpenCount", open.count == 11)
check("rowsProviderRows", open[4].kind == .provider(id: "a") && open[4].checked == true && open[5].checked == false && open[4].indented)
check("rowsShowCard", open[7].kind == .showCard(id: "h") && open[7].title == "Show Codex")
check("rowsSpinning", open[0].spinning)
check("cardsSkipped", PanelModel.cards(entries).isEmpty)

// highlight
let enabled = [true, false, true, false]
check("highlightFirst", PanelModel.highlight(from: nil, step: 1, enabled: enabled) == 0)
check("highlightLast", PanelModel.highlight(from: nil, step: -1, enabled: enabled) == 2)
check("highlightSkips", PanelModel.highlight(from: 0, step: 1, enabled: enabled) == 2)
check("highlightNoWrap", PanelModel.highlight(from: 2, step: 1, enabled: enabled) == 2)
check("highlightNone", PanelModel.highlight(from: nil, step: 1, enabled: [false]) == nil)

// layout
let stacked = PanelLayout.stack([10, 20, 5])
check("stackTops", stacked.tops == [0, 10, 30] && stacked.total == 35)
check("stackEmpty", PanelLayout.stack([]).total == 0)
check("heightCapped", PanelLayout.height(content: 900, available: 600) == 600 && PanelLayout.height(content: 300, available: 600) == 300)
let screen = CGRect(x: 0, y: 0, width: 1000, height: 800), size = CGSize(width: 320, height: 400)
check("originUnderAnchor", PanelLayout.origin(anchor: CGRect(x: 100, y: 776, width: 30, height: 24), size: size, screen: screen) == CGPoint(x: 100, y: 372))
check("originClampsRight", PanelLayout.origin(anchor: CGRect(x: 950, y: 776, width: 30, height: 24), size: size, screen: screen).x == 672)
check("originOtherScreen", PanelLayout.origin(anchor: CGRect(x: -500, y: 776, width: 30, height: 24), size: size,
                                              screen: CGRect(x: -1000, y: 0, width: 1000, height: 800)).x == -500)

// tween: build cards with CardModel.card(...) from sampleRun, or make small Card values if an init is reachable
check("tweenValueClamped", Tween.value(0, 10, 2) == 10 && Tween.value(0, 10, -1) == 0)
check("tweenIntroFromZero", /* old nil, new ring 0.4, progress 0.5 */ == [[0.2]])
check("tweenStartsFromScreen", /* old ring 0.2, new 0.6, progress 0.5 */ == [[0.4]])
check("tweenRingAdded", /* old 1 ring 0.5, new 2 rings 0.5 and 0.8, progress 0.5 */ == [[0.5, 0.4]])
check("tweenChartRemoved", /* old 2 charts, new 1 chart */ .count == 1)
check("tweenNilStaysNil", /* new ring fraction nil */ == [[nil]])
check("percentCounts", Tween.percent(from: "3%", to: "13%", progress: 0.5) == "8%")
check("percentIntro", Tween.percent(from: nil, to: "40%", progress: 0.5) == "20%")
check("percentOtherText", Tween.percent(from: "3%", to: "63% · 6280/10000 credits", progress: 0.5) == "63% · 6280/10000 credits")
```

The tween checks need `Card` values. `Card`, `CardChart` and `CardRow` have no public init today: add `public init` memberwise to `CardChart` and `Card` in `Sources/AIUsageCore/CardModel.swift` (rows can be `[]` in these checks), and replace the `/* */` placeholders with real expressions.

- [ ] **Step 2: Run to see them fail**

Run: `swift run CoreChecks`
Expected: build error, `cannot find 'PanelModel' in scope`.

- [ ] **Step 3: Implement `PanelModel`, `PanelLayout`, `Tween`** in `Sources/AIUsageCore/PanelModel.swift` with the signatures above. `import Foundation` gives `CGFloat`, `CGRect`.

- [ ] **Step 4: Run to see them pass**

Run: `swift run CoreChecks 2>&1 | tail -3`
Expected: `ALL PASS`

---

### Task 2: `RowView`

**Files:**
- Create: `Sources/AIUsage/RowView.swift`

**Interfaces:**
- Consumes: `PanelRow` (Task 1).
- Produces:

```swift
enum Motion {
    /// `seconds`, or 0 with Reduce Motion on.
    static func duration(_ seconds: TimeInterval) -> TimeInterval
    /// Runs `changes` in an ease-out `NSAnimationContext` group of `duration(seconds)` with `allowsImplicitAnimation`.
    static func animate(_ seconds: TimeInterval, _ changes: () -> Void, completion: (() -> Void)? = nil)
}

final class RowView: NSView {
    static func height(_ row: PanelRow) -> CGFloat          // 22; separator 11
    static func width(for rows: [PanelRow]) -> CGFloat      // widest title + shortcut + check + padding
    init(row: PanelRow, onClick: @escaping (PanelRow.Kind) -> Void)
    private(set) var row: PanelRow
    func update(_ row: PanelRow)        // check mark fades 0.12 s; spinner starts or stops; redraws
    var highlighted: Bool               // keyboard or pointer highlight; setter redraws
    var onHover: ((RowView) -> Void)?   // pointer entered: the panel moves the highlight here
}
```

`Motion` lives at the top of `RowView.swift` (first file that needs it; `CardView` and `PanelController` use it later).

Drawing (flipped view, `wantsLayer = true`):
- Text starts 14 pt from the left, 14 pt more when `indented`. Font `NSFont.menuFont(ofSize: 0)`. Color: `row.tone` mapped as `StatusController.color(_:)` does today, `.disabledControlTextColor` when not `enabled`, `.selectedMenuItemTextColor` when highlighted.
- Highlight: `NSColor.selectedContentBackgroundColor` rounded rect, 5 pt inset left and right, 4 pt radius, only when `highlighted && row.enabled`.
- Pressed (mouse down inside, enabled): highlight at 0.8 alpha, set at once, no animation.
- Trailing, 14 pt from the right: shortcut text in `.secondaryLabelColor`, or the `checkmark` SF Symbol (11 pt, medium) when `checked == true`, or a chevron (`chevron.right`, rotated 90° when `expanded == true`, rotation animated 0.22 s) when `expanded != nil`.
- `spinning`: an `NSProgressIndicator` (`.spinning`, `controlSize = .small`, `isDisplayedWhenStopped = false`) where the shortcut is; the shortcut hides while it spins.
- `.credit`: draw `before`, the `heart.fill` symbol tinted `.disabledControlTextColor`, `after` (move the tinting code from `StatusController.item(for:)`).
- `.separator`: 1 pt line, `NSColor.separatorColor`, 14 pt inset.
- Mouse up inside an enabled row calls `onClick(row.kind)`.
- Accessibility: role `.button`, or `.checkBox` with value when `checked != nil`; label = title; disabled rows are static text; `accessibilityPerformPress` calls `onClick`.

- [ ] **Step 1: Implement `Motion` and `RowView`** as specified.

- [ ] **Step 2: Build**

Run: `swift build 2>&1 | tail -3`
Expected: `Build complete!`

No Core check: this task is drawing only; its logic (`PanelRow`) was checked in Task 1. It is exercised by hand in Task 6.

---

### Task 3: `CardView` takes updates and animates

**Files:**
- Modify: `Sources/AIUsage/CardView.swift`

**Interfaces:**
- Consumes: `Tween.fractions`, `Tween.percent` (Task 1), `Motion` (Task 2).
- Produces:

```swift
// existing init: unchanged signature
func update(card: Card, collapsed: Bool, canMoveUp: Bool, canMoveDown: Bool, animated: Bool)
var fittingHeight: CGFloat { get }   // render(draw: false) for the current state
func playIntro()                     // rings and percents from 0, 0.5 s
```

Changes:
- `card`, `collapsed`, `canMoveUp`, `canMoveDown` become `private(set) var`. `init` sets `wantsLayer = true`, `layerContentsRedrawPolicy = .onSetNeedsDisplay`, `layerContentsPlacement = .topLeft` (the panel animates the frame height; content must not stretch).
- Three animatable properties, each `@objc dynamic var … : CGFloat { didSet { needsDisplay = true } }`, with `override class func defaultAnimation(forKey:)` returning a `CABasicAnimation` for their keys:
  - `progress` (default 1): position between `previous: Card?` and `card`. `drawChart` draws `Tween.fractions(from: previous, to: card, progress:)` instead of `ring.fraction`; the chart center, row pills and collapsed pills draw `Tween.percent(from: old text, to: new text, progress:)`, old text matched by chart, row and pill index. Pill tint and text color: blend old tone color to new by `progress` (`NSColor.blended(withFraction:of:)`).
  - `hover` (default 0): replaces the `cardHovered` Bool in drawing. Outline color blends `.separatorColor` to `.tertiaryLabelColor` by `hover`; hover-only buttons draw at alpha `hover`; collapsed pills draw at alpha `1 - hover`. Hit testing still uses the Bool. Enter and exit set `animator().hover` over 0.12 s.
  - `turn` (0 collapsed, 1 expanded): the chevron is one shape (the right-pointing one) rotated by `turn * 90°`.
- `update(...)`: if `animated` and the card differs, set `previous` to a snapshot of what is on screen: when an animation is running (`progress < 1`), that is not the old `card` but the in-between state, so keep the on-screen fractions (`Tween.fractions(from: previous, to: card, progress: progress)`) in a `[[Double?]]` field and tween from it. Then store the new values, set `progress = 0`, and `Motion.animate(0.4) { animator().progress = 1 }`. A change of `collapsed`: `Motion.animate(0.22) { animator().turn = collapsed ? 0 : 1 }` and add a 0.22 s `CATransition` fade (`Motion.duration`) to the layer so charts fade. A change of pin: 0.15 s `CATransition` fade. Not `animated`: set values, `progress = 1`.
- `playIntro()`: `previous = nil`, on-screen fractions cleared, `progress = 0`, `Motion.animate(0.5) { animator().progress = 1 }`.
- Pressed state stays instant (`setState` already redraws at once).
- Accessibility: `summary` and the children read the current `card`; call `setAccessibilityLabel` again in `update`.
- Update the class doc comment: the view is kept and updated, not replaced.

- [ ] **Step 1: Make the changes above.**

- [ ] **Step 2: Build the app and the picture script** (the script proves the init and the never-animated drawing still work)

Run:
```bash
swift build 2>&1 | tail -3
mkdir -p build && grep -v '^import AIUsageCore$' Sources/AIUsage/CardView.swift > build/CardView.swift && grep -v '^import AIUsageCore$' Sources/AIUsage/RowView.swift > build/RowView.swift && swiftc -parse-as-library -o build/make-panel Sources/AIUsageCore/*.swift build/CardView.swift build/RowView.swift scripts/make-panel.swift && echo OK
```
Expected: `Build complete!` then `OK`. `CardView` now needs `Motion` from `RowView.swift`, so the script's build line gains `build/RowView.swift`: update the comment at the top of `scripts/make-panel.swift` and the same two lines in `README.md` (near line 132).

- [ ] **Step 3: Run the checks**

Run: `swift run CoreChecks 2>&1 | tail -1`
Expected: `ALL PASS`

---

### Task 4: `PanelController`

**Files:**
- Create: `Sources/AIUsage/PanelController.swift`

**Interfaces:**
- Consumes: `PanelRow`, `PanelModel.highlight`, `PanelLayout` (Task 1); `RowView`, `Motion` (Task 2); `CardView.update`, `fittingHeight`, `playIntro` (Task 3).
- Produces:

```swift
struct PanelContent {
    var cards: [Card]
    var collapsed: Set<String>
    var rows: [PanelRow]
}

struct PanelActions {
    var pin: (String) -> Void
    var hide: (String) -> Void
    var move: (String, Int) -> Void
    var toggle: (String) -> Void
    var row: (PanelRow.Kind) -> Void
}

final class PanelController: NSObject {
    init(actions: PanelActions)
    private(set) var isOpen: Bool
    var onClose: (() -> Void)?
    func open(under button: NSStatusBarButton, content: PanelContent)
    func update(_ content: PanelContent)   // no-op when closed
    func close()
}
```

Window:
- `NSPanel` subclass with `canBecomeKey` true; style `[.borderless, .nonactivatingPanel]`, `level = .popUpMenu`, `isOpaque = false`, `backgroundColor = .clear`, `hasShadow = true`, `collectionBehavior = [.transient, .ignoresCycle, .fullScreenAuxiliary]`.
- Content: `NSVisualEffectView` (`material = .menu`, `state = .active`, layer corner radius 10, `masksToBounds`), holding an `NSScrollView` (no background, vertical scroller automatic, overlay style) whose document view is a flipped `NSView`. 5 pt padding above the first and below the last view.

Layout:
- Width: `max(CardView.width(for: cards, leading: 14), RowView.width(for: rows))`. Card `leading` is 14 always (there is no menu check column now).
- Views top to bottom: cards in order, then rows. Tops from `PanelLayout.stack`. Panel height from `PanelLayout.height(content:available:)`, `available` = the icon's screen `visibleFrame.height` minus 8. Position from `PanelLayout.origin(anchor: button's frame in screen coordinates, size:, screen: visibleFrame)`.

`update(_:)` (also used by `open` with no animation):
- Cards are matched by `card.id`, rows by `row.kind`.
- Kept view: call `update(...)` on it, `animator().frame` to its new place over 0.22 s. This one rule gives collapse (height), reorder (two cards trade tops) and the slide of everything below.
- Removed view: `animator().alphaValue = 0` over 0.15 s, then remove from superview; the rest slide after it (0.22 s, started in the fade's completion).
- New view: placed at its final frame with `alphaValue = 0`; the others slide 0.22 s, then it fades in 0.15 s. A new card also gets `playIntro()`.
- The window frame animates to the new height in the same 0.22 s group, top edge fixed.
- If the width changes, rebuild all views without animation (width changes are rare: a new longer label).
- A second `update` during an animation just runs again: `animator()` starts from the presentation value.

Open and close:
- `open`: build with no animation, `alphaValue = 0`, frame 6 pt above its place; `makeKeyAndOrderFront`; animate alpha to 1 and frame down over 0.15 s; every card `playIntro()`.
- `close`: fade to 0 over 0.1 s, then `orderOut`, remove monitors, drop all views (memory), call `onClose`. `isOpen` turns false at the start of `close`, so `update` during the fade is a no-op.
- While open: one `NSEvent.addGlobalMonitorForEvents` and one local monitor for `[.leftMouseDown, .rightMouseDown]`; a click outside the panel closes it, except a click on the status button (the button's own action toggles). `windowDidResignKey` closes too. Monitors are removed in `close`.

Keyboard (`keyDown` / `performKeyEquivalent` on the panel subclass, forwarded to the controller):
- Esc: `close()`. `⌘R`: `actions.row(.refresh)`. `⌘Q`: `actions.row(.quit)`.
- Up, Down: `PanelModel.highlight(from:step:enabled:)` over the rows; set `highlighted` on the row views; scroll the row into view. Return or Space: `actions.row(kind)` of the highlighted row.
- Pointer entering a row (`onHover`) moves the highlight there; leaving all rows clears it.

- [ ] **Step 1: Implement `PanelController`** as specified.

- [ ] **Step 2: Build**

Run: `swift build 2>&1 | tail -3`
Expected: `Build complete!`

---

### Task 5: Wire `StatusController` to the panel, remove the menu

**Files:**
- Modify: `Sources/AIUsage/StatusController.swift`
- Modify: `docs/specs/menubar/menubar-panel.md`, `README.md` (line 121: `StatusController` (icon, actions), `PanelController` (panel), `RowView`)

**Interfaces:**
- Consumes: `PanelController`, `PanelContent`, `PanelActions` (Task 4); `PanelModel.cards`, `PanelModel.rows` (Task 1).

Changes:
- Remove: `menu`, `NSMenuDelegate`, `menuWillOpen`, `menuDidClose`, `menuNeedsUpdate`, `item(for:position:)`, `action(_:_:key:)`, `color(_ tone:) -> NSColor?`, `cardWidth`, `cardLeading`, `menuOpen`, `menuRebuildPending`, `rebuildOpenMenu`.
- Add `private var expandedSections: Set<String> = []` (Providers and Hidden Cards open state; cleared in `panel.onClose`).
- Add `private func content() -> PanelContent`: the same `MenuModel.entries(...)` call as today, then `PanelModel.cards(entries)`, `collapsedCards`, `PanelModel.rows(entries, expanded: expandedSections, refreshing: poller.isRunning)`.
- Add `private func refreshPanel() { panel.update(content()) }`; call it where `rebuildOpenMenu()` was called (`pollerChanged`, the four card actions) and at the end of every row action below.
- Status button: `button.target = self`, `button.action = #selector(togglePanel)`; `togglePanel` closes when `panel.isOpen`, else `panel.open(under: button, content: content())`. While open, `button.highlight(true)`; `onClose` sets it back.
- `PanelActions.row` switch: `.refresh` → `refresh()`; `.providers` / `.hiddenCards` → toggle the section id in `expandedSections`; `.provider(id)` → today's `toggleProvider` body; `.showCard(id)` → `hiddenCards.remove(id)`; `.launchAtLogin` → `toggleLaunchAtLogin()`; `.openProvidersFolder` → `panel.close()` then open the folder; `.quit` → `quit()`. Others: nothing. All but the last two end with `refreshPanel()`.
- The `@objc` action methods that took an `NSMenuItem` become plain methods taking the id.
- `docs/specs/menubar/menubar-panel.md`: replace "A standard AppKit menu" and the `NSMenu` notes (lines 9 to 16, 79, 107) with the panel: stays open on every click except Open Providers Folder and Quit; Providers and Hidden Cards open in place; card content is 14 pt in always; the motion table from the design; keyboard keys; how it closes. Line 46 (relative text is computed on open) still holds.

- [ ] **Step 1: Make the changes above.**

- [ ] **Step 2: Build and run the checks**

Run: `swift build 2>&1 | tail -3 && swift run CoreChecks 2>&1 | tail -1 && grep -n "NSMenu" Sources/AIUsage/*.swift`
Expected: `Build complete!`, `ALL PASS`, and no `NSMenu` matches.

---

### Task 6: Verify in the real app

**Files:** none, unless a step fails.

- [ ] **Step 1: Install and start**

Run: `scripts/build-app.sh && pkill -x AIUsage; open "$HOME/Applications/AI Usage.app"`
Expected: `Installed …/AI Usage.app`; the icon appears in the menu bar.

- [ ] **Step 2: Memory, panel open**

Run (with the panel open): `footprint -p $(pgrep -x AIUsage) | grep phys_footprint`
Expected: under 30 MB. Close the panel, wait 10 s, run `top -l 2 -pid $(pgrep -x AIUsage) -stats cpu | tail -1`: 0.0 (nothing runs while idle).

- [ ] **Step 3: By hand, each line once, in light and dark**

1. Open: fade and slide; rings sweep, percents count up.
2. Refresh Now: panel stays; spinner and `Refreshing…`; then `Updated <time>`; changed values move.
3. Providers: opens in place; a check box click stays open, check fades, the card goes or comes.
4. Launch at Login: stays open, check fades.
5. Collapse and expand: height animates, chevron turns, cards below slide. Click twice fast: ends in the right state.
6. Move up, then down at once: cards slide and end in the right order.
7. Hide: card fades, others slide up; Hidden Cards row appears; Show brings it back with a fade.
8. Pin another value: `Pinned` crossfades; the menu bar icon changes.
9. Hover a card: outline and buttons fade. Press a button: darkens at once.
10. Keyboard only: Up, Down, Return, Space, `⌘R`, Esc, `⌘Q`.
11. Close by: click outside, click the icon, switch app.
12. Start a refresh, close the panel before it ends, reopen after: current values, no crash.
13. Collapse nothing and enable every provider so the panel is taller than the screen (or test on the smallest display): it scrolls; Quit is reachable.
14. Icon near the right screen edge (hide other menu bar items or use a narrow display): the panel is fully on screen.
15. System Settings, Accessibility, Display, Reduce Motion on: everything from 1 to 9 happens with no animation.
16. VoiceOver (`⌘F5`): rows read as buttons or check boxes with state; card buttons are reachable.

- [ ] **Step 4: New screenshots**

Run the two `make-panel` lines from Task 3 Step 2, then `build/make-panel` as `README.md` describes, and check `docs/images/panel.png` still shows the cards. The rows under the cards in that picture are drawn by the script, not by the app; leave them unless they now look different from the app.
