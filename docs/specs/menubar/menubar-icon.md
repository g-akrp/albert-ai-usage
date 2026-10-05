# Menu bar icon

The icon uses a pixel style with two rows: the provider name on top, the usage value on the bottom. It is drawn with a built-in 3×5 pixel font (uppercase letters, digits, `%`, `-`, `.`, `?`).

## Top row

- The provider's `iconLabel` (for example `CLD`, `CDX`, `GHC`, `AGY`) in the provider's `iconColor`. Without `iconLabel`, the first three letters of the id; without `iconColor`, gray.
- GitHub Copilot with more than one account uses `GH1`, `GH2`, … in the order `gh` lists the accounts.
- When a single limit is pinned, the first three letters or digits of that limit's name, for example `GEM`, in the provider's color.

## Bottom row: the value

The number shown for a provider is its headline percent, the first of these that the provider reports:

1. Session limit (5-hour window, or a window labeled session / five hour).
2. Weekly limit (7-day window, or a window labeled weekly).
3. Premium interactions (a limit whose id or label contains "premium").
4. Otherwise the highest used percent across all windows.

Within one kind the highest percent wins (for example across Antigravity's model groups). A pinned limit shows that limit's own highest window percent, not the headline.

## Color of the value

Color follows the highest used percent across all windows of the provider, not the headline value, so a nearly full weekly limit stays visible while the number shows the session:

- Green below 70%.
- Orange from 70%.
- Red from 90%.

## Special states

- A provider that fails shows a red `ERR`.
- A pinned provider with no usage data shows `--` in gray.
- The accessibility text and tooltip name the provider, the limit when pinned, and the value, for example `Claude 10%` or `Antigravity Gemini Models 7%`.

## Pin

- There is no cycle mode. The icon always shows one pinned target, set from the [panel](menubar-panel.md).
- Pinned to a provider: that provider's icon.
- Pinned to a limit: that limit's icon.
- No valid pin (first launch, upgrade, or the pinned provider or limit is gone or off): the first provider that is on is pinned and the pin is saved. Antigravity can pin only a group, so its default is its first group. A pin on `copilot` falls back to its first account. A pinned limit that disappears falls back to its provider, then to the default.
- A pinned provider that has not finished its first run shows `--` in gray. If no provider is on, the icon shows `AI` over `--` in gray.
- Pin is kept across launches (user default `pinnedProvider`). Format: run id (`codex`, `copilot:octocat`), or run id and limit id joined by `|` (`antigravity|Gemini Models`).
