# Own panel with motion: design

Date: 2026-11-13. Status: waiting for approval.

## Intent

Two complaints about the panel that opens from the menu bar icon:

1. It closes when you click Refresh Now, a Providers check box, Launch at Login, or a hidden card's Show item. It should stay open.
2. It feels static. Everything snaps: values, collapse, reorder, hide, hover.

The panel is an `NSMenu` today. `NSMenu` closes on any standard item click and resizes in jumps, so both complaints have the same cause.

## Goals

- Clicking Refresh Now, a provider check box, Launch at Login, or Show (hidden card) leaves the panel open and shows the result in place.
- Motion and feedback on every change the user causes or a refresh brings (list below).
- Same information, same card layout, same settings as today.

## Non-goals

- No new card layout, colors, or fonts. The cards look as in `docs/images/panel.png`.
- No change to providers, polling, the menu bar icon, or stored settings keys.
- No drag to reorder (the arrow buttons stay).

## Anti-goals

- Motion that delays reading a value: every number is readable within 0.5 s of the panel opening.
- Any timer that runs while nothing animates, or memory above 30 MB (`AGENTS.md`).
- Losing keyboard or VoiceOver access that the menu gave for free.

## Constraints

- Swift, AppKit only, macOS 13+, no packages (`AGENTS.md`).
- Logic in `Sources/AIUsageCore/`, covered by `swift run CoreChecks`, check written first.
- Reduce Motion (`NSWorkspace.shared.accessibilityDisplayShouldReduceMotion`): every duration is 0.

## Approaches

| | A. Keep `NSMenu`, custom row views | B. Own panel (picked) |
|---|---|---|
| Stay open on click | Yes, but submenus must become inline rows | Yes |
| Ring sweep, hover and press fades, spinner | Yes | Yes |
| Smooth collapse, reorder slide, hide fade, open transition | No: `NSMenu` resizes in jumps | Yes |
| Cost | Small diff | Rewrite of the menu half of `StatusController.swift`; we own closing, keyboard, VoiceOver, placement |

A third way, a private `NSMenu` flag to keep it open, was dropped: private API.

The user picked B: most of the wanted motion is what `NSMenu` cannot do.

## Design

### Window

- A borderless, non-activating `NSPanel` at `.popUpMenu` level that can become key, holding an `NSVisualEffectView` (material `.menu`, 10 pt corners) so it looks like the menu it replaces.
- The status item has no `menu`; its button's action toggles the panel. The panel sits under the icon, right edge kept on screen. Taller than the screen: the content scrolls (`NSScrollView`).
- Closes on: click outside (one global and one local event monitor, installed while open only), Esc, a second click on the icon, the app losing focus, Open Providers Folder, Quit.

### Content

Top to bottom, as today: cards, then rows.

- **Cards:** `CardView` stays one view per card with its `draw(_:)`. It stops being rebuilt on every change: the panel keeps one `CardView` per card id and gives it the new `Card`, so a view can animate from what it shows to what it should show. Views are added and removed only when a card appears or goes.
- **Rows:** one new `RowView` for every non-card line: title, optional check mark, optional shortcut hint (`⌘R`, `⌘Q`), optional spinner, disabled look for `Updated …`, version and credit. Hover draws the menu's rounded selection highlight.
- **Providers** and **Hidden Cards** are rows with a chevron that open in place into indented rows (a check box per provider, `Show <name>` per hidden card). No submenus: a child window would bring back the closing problem. (My call.)

### Motion

Standard curve: ease-out. All through `NSAnimationContext` and the `animator()` proxy on layer-backed views; ring and number tweening uses a custom animatable property on `CardView` (`defaultAnimation(forKey:)`), so AppKit drives the frames and nothing runs when idle.

| Event | What moves | Duration |
|---|---|---|
| Panel opens | Fade in, 6 pt slide down | 0.15 s |
| Panel opens | Rings sweep from 0 to their value, center percent counts up | 0.5 s |
| Panel closes | Fade out | 0.1 s |
| Refresh brings a new value | Ring, center percent and pill move from old to new; pill tint crossfades when the tone changes | 0.4 s |
| Refresh Now clicked | Row shows a spinner and `Refreshing…` until every run it started ends; then `Updated <time>` | until done |
| Collapse or expand | Card height animates, charts fade, chevron rotates, cards below slide | 0.22 s |
| Move up or down | The two cards slide past each other | 0.22 s |
| Hide | Card fades out, then cards below slide up | 0.15 + 0.22 s |
| Show a hidden card | Space opens, card fades in | 0.22 + 0.15 s |
| Pointer enters or leaves a card | Outline and buttons fade in or out | 0.12 s |
| Button or row pressed | Fill darkens at once (no animation: press feedback must be instant) | 0 |
| Pin | `Pin` crossfades to `Pinned` in the accent color; the old pin fades back | 0.15 s |
| Check box row clicked | Check mark fades in or out | 0.12 s |
| Launch at Login fails | Beep, as today | - |

A new change during an animation starts from the value on screen, not from the animation's end.

### Keyboard and VoiceOver

- `⌘R` refreshes, `⌘Q` quits, Esc closes.
- Up and Down move a highlight through the rows; Return or Space acts on the highlighted row. Cards are not in this order (they were not selectable in the menu either).
- Cards keep their accessibility children (pin, hide, move, toggle). Each `RowView` is a button, or a check box with its state.

### Code

- `Sources/AIUsageCore/`: `MenuModel.entries` keeps producing the entry list. New pure logic, each with a check first:
  - stacking: heights in, y positions out (used for the slide animations);
  - tween: old and new `Card` plus progress in, the ring fractions and percent texts to draw out (rings matched by position; a ring with no old value starts at 0);
  - row highlight: which row Up and Down land on, skipping disabled rows.
- `Sources/AIUsage/PanelController.swift` (new): the window, placement, closing, keeping card and row views in step with the entries.
- `Sources/AIUsage/RowView.swift` (new).
- `Sources/AIUsage/CardView.swift`: takes a new `Card` after creation, animatable progress, hover fade. Layout code unchanged.
- `Sources/AIUsage/StatusController.swift`: loses the `NSMenu` and item building; keeps the icon, settings and actions.
- `docs/specs/menubar/menubar-panel.md`: updated to describe the panel.

## Left to the builder

- Exact names of the new types and functions.
- Whether the three pure functions live in one file or in `MenuModel.swift`.
- How the open transition is split between window alpha and content offset.

## Checks before done

- `swift run CoreChecks` passes.
- `footprint -p $(pgrep -x AIUsage) | grep phys_footprint` under 30 MB with the panel open.
- By hand: each row of the motion table, with Reduce Motion on and off, light and dark, and keyboard only.
