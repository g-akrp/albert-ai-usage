# Menu bar panel

This is the spec for the panel that appears when you click the menu bar icon.

**Status: implemented.** A card per provider replaced the plain text menu with `☆`/`★` stars; cycle mode is removed.

## Layout

A standard AppKit menu, top to bottom:

1. One card per provider run that is on, each in a thin rounded outline (system separator color, a little darker while the pointer is over the card). The outline is 10 pt from the menu's left and right edges. Cards sit 8 pt apart with no separator line between them; one separator follows the last card. Content is 12 pt further in than the other items' text on both sides (26 or 34 pt from the menu edge, see below), which is 16 or 24 pt inside the outline's sides, and 16 pt inside its top and bottom. Every card has the same width: the widest content of any card, at least 300 pt, so nothing is clipped. Cards are transparent: the menu's own background shows through. GitHub Copilot gets one card per `gh` account. Providers that are off are hidden.
2. A separator, then the normal items: configuration errors, `All providers are off` when every provider is off, **Refresh Now** (⌘R), `Updated <time>`, **Providers** submenu, **Hidden Cards** submenu (only when a card is hidden), **Launch at Login**, **Open Providers Folder…**, the version as `AI Usage <version>`, the credit line `Built with ♥ by g.akrp` (the heart is the SF Symbol `heart.fill`, not an emoji, in the same gray as the text), **Quit**. **Cycle All Providers** is removed.

## Card

Card content starts at the same left edge as the text of the other menu items: 14 pt, or 22 pt while **Launch at Login** is on (macOS then reserves a column for its check mark and moves every item's text right; measured from the open menu).

```
┌──────────────────────────────────────────────────┐
│  📌 Claude Code (max)                            │
│   ╭───────╮                                      │
│  ╭│ ╭───╮ │╮   ● Session   10%                   │
│  ││ │10%│ ││      Oct 5, 3:20 PM · in 2 hours    │
│  ╰│ ╰───╯ │╯   ● Weekly    80%                   │
│   ╰───────╯      Oct 9, 9:00 AM · in 4 days      │
└──────────────────────────────────────────────────┘
```

- **Title row:** provider name and plan in parentheses on the left, with the account name (GitHub Copilot) on a gray line under it; the `Hide` and `Pin` buttons on the right.
- **Left:** a concentric radial bar chart. **Right:** one row per ring, with the reset time under it.
- **Failed:** the title and the error text in red, no chart. **Loading:** the title and `Loading…`. A provider with no limits shows `No usage data`. `Usage limit reached` (red) and `Plan limits don't apply to this account` are shown under the title as today.

### Chart

- One chart per meter. One ring per window of that meter, outermost is the highest priority: session (5 h), then weekly (7 d), then any other window in file order.
- A ring starts at 12 o'clock and sweeps clockwise to its used percent, over a faint track. Ring color: the provider's brand color (`iconColor`) on the outermost ring, each ring inside it lighter (blended with white by 0, 30, 50, 65%). The row's dot has its ring's color. The usage warning stays on the percent number: green below 70%, orange from 70%, red from 90%.
- The center shows the chart's headline percent: session, else weekly, else the highest.
- At most 4 rings per chart. Extra windows are listed as text rows with no ring.
- A provider whose meters each have one unlabeled window (GitHub Copilot: Chat, Completions, Premium Interactions) merges them into one chart with a ring per meter. Premium Interactions is outermost, then Chat, then Completions.

### Rows

- Each ring has a row: a dot in the ring's color, the label, and the used percent in a pill at the right (rounded, tinted with the percent's tone at 16% opacity, text in the tone color) so the value reads on any menu background.
- Spacing: 14 pt side margin, 12 pt top and bottom, 20 pt text lines, 8 pt between rows, 14 pt between charts, 4 pt between a header and what follows. A chart's rows are vertically centered against its rings when they are shorter than the rings.
- Under it: the reset time in the local time zone, a middle dot, and the time until it, for example `Oct 5, 3:20 PM · in 2 minutes`. Relative units: seconds, minutes, hours, days (`in 1 minute`, `in 3 hours`, `in 4 days`). A reset time in the past shows `resetting…`. A window with no reset time (Copilot) shows no second line.
- The relative text is computed when the menu opens. It does not update while the menu stays open.

### Provider with several groups (Antigravity)

One chart per model group, stacked, each with its own group header and rings for its windows. A group with one window has one ring. Up to 4 groups get a chart; further groups are listed as text rows.

```
┌──────────────────────────────────────────────────┐
│  📌 Antigravity                                  │
│   ╭─────╮    Gemini Models                  📌   │
│  ╭│╭───╮│╮    ● Five Hour  3%                    │
│  ││╰ 3%╯││      Oct 5, 3:20 PM · in 2 hours       │
│  ╰╰─────╯╯    ● Weekly     7%                    │
│                 Oct 9, 9:00 AM · in 4 days       │
│  ────────────────────────────────────────────    │
│   ╭─────╮    Claude and GPT models          📌   │
│  ╭│ 95% │╮    ● Weekly    95%                    │
│  ╰╰─────╯╯      Oct 9, 9:00 AM · in 4 days       │
└──────────────────────────────────────────────────┘
```

## Card order

- Default order: Claude Code, Codex, Antigravity, GitHub Copilot (by provider file id `claude`, `codex`, `antigravity`, `copilot`); any other provider follows, in id order. Copilot accounts keep the order `gh` lists them. The Providers submenu and the icon's default pin follow the same order.
- While the pointer is over a card, two outlined arrow buttons, `↑` and `↓`, show in its title row, left of **Hide**. Clicking one swaps the card with its nearest visible neighbor and closes the menu. The top card has no `↑`; the bottom card has no `↓`. Hidden cards are stepped over and keep their place.
- The order is saved by run id (user default `cardOrder`). A card not in the saved list, such as a new Copilot account, goes after the saved ones in default order. Saved ids that no longer exist are ignored.
- There is no drag and drop: an `NSMenu` cannot reorder by dragging.

## Hide a card

- While the pointer is over a card, an outlined text button labeled `Hide` (same style, hover and press feedback as the pin) shows in its title row, left of the pin. Clicking it hides that card and closes the menu.
- Hiding is per card: one Copilot account can be hidden while the other stays. It only removes the card from the panel. The provider keeps running, so it can still be pinned and shown in the icon, and the cycle of results is unaffected.
- **Hidden Cards** submenu: one `Show <name>` item per hidden card that still exists. Choosing it shows the card again. The ids of hidden cards are kept across launches (user default `hiddenCards`); an id with no matching card (for example a removed account) is ignored and kept.
- When every card is hidden the panel shows `All cards are hidden`.
- This is different from the **Providers** submenu, which turns monitoring of a whole provider file on or off.

## Pin

There is no cycle mode. The icon always shows exactly one pinned target (see [menu bar icon](menubar-icon.md)).

- Each pin sits in a small outlined rounded box (a button): the outline is gray, accent colored when pinned. Hovering a pin (or its title row) fills the box lightly, darkens its outline and shows a pointing-hand cursor; pressing fills it darker. The pin is set on mouse up inside the same box area.
- Pins are small outlined text buttons, like **Hide**. The current pin always shows as `Pinned` (accent color outline and text, and the card title is bold). Any other pin shows as `Pin` only while the pointer is anywhere over the card (all of the card's pins and **Hide** appear together), so the card stays clean otherwise. Hovering fills the button lightly, darkens its outline and shows a pointing-hand cursor; pressing fills it darker. The pin is set on mouse up inside the same button area.
- The card's title-row pin sits at the right end of the row, after **Hide**, in line with the limit and group pins below it.
- Clicking a pin pins that target and releases the previous one. A pinned target cannot be unpinned directly: pin another one instead.
- **Default pin:** when there is no valid pin (first launch, an upgrade from cycle mode, or the pinned provider or limit no longer exists or is switched off), the first provider that is on, in panel order, is pinned. The default is written back so the stored pin is always valid. If no provider is on, the icon shows `AI` over `--` in gray.
- Where a provider has several pinnable limits, each has its own pin and pins just that limit. The icon then shows the first three letters of its name, for example `GEM`.
- **Antigravity can pin only a group, never the whole provider.** Its title pin pins its first group (Gemini Models) and is filled when `antigravity|Gemini Models` is the pin. Each group header has its own pin. If Antigravity is the first provider, the default pin is its first group.
- Claude and Codex pin the provider. Copilot pins the provider or a single limit.
- The pin is kept across launches. Format is unchanged: `run` or `run|meter`.
- Each pin is an accessibility button labeled `Pin Claude Code`, or `Unpin Claude Code` for the current pin.

## Implementation notes

- Card model, ring order, rows, relative-time text and default pin resolution live in `Sources/AIUsageCore/` with `CoreChecks` written first. The relative-time function takes `now` as an input.
- `CardView` (`NSView`, set as `NSMenuItem.view`) draws the rings with Core Graphics and handles clicks. A click runs the pin action and closes the menu. System colors, so light and dark mode work.
- Removing cycle mode also removes the 5 s icon cycle timer; only the 30 s schedule timer remains. Views are rebuilt each time the menu opens. Memory footprint must stay under 30 MB.
- Each card has an accessibility label summarizing the provider and its values.
