# Menu bar panel

This is the spec for the panel that appears when you click the menu bar icon. It is a standard AppKit menu, top to bottom:

## Providers

One block per provider that is on, separated by lines. For GitHub Copilot, one block per `gh` account.

- Title row: `☆ Name (plan)`. `★` when pinned. Clicking it pins the provider.
- A provider still loading shows `Loading…`. A failed provider shows `Error: <reason>` in red.
- `Usage limit reached` in red when the provider reports no ordinary usage allowed. `Plan limits don't apply to this account` when the plan has no limits. `No usage data` when there are no limits.
- Each limit window: `Label: 42% (resets Oct 2, 2026 6:23 PM)`, with the reset time in the local time zone. Window titles come from the provider, or from the window length: Session (5 h), Weekly (7 d), Monthly.
- The percent is colored green below 70%, orange from 70%, red from 90%.
- A provider with several limits (Antigravity's model groups, Copilot's Chat, Completions, Premium Interactions) lists each with its own `☆`. Clicking one pins just that limit.

## Below the providers

- Configuration errors: one red line per provider file that did not load, with the reason.
- `All providers are off` when every provider is switched off.
- **Cycle All Providers**: checked when nothing is pinned; choosing it unpins.
- **Refresh Now** (⌘R): runs all providers right away.
- `Updated <time>`: when the last result arrived.
- **Providers**: submenu with one checkbox per provider file. Off means it never runs and is hidden from the icon and the panel; turning it on runs it right away. Kept across launches.
- **Launch at Login**: checkbox.
- **Open Providers Folder…**: opens `~/.config/ai-usage/providers/`.
- Installed version, then **Quit**.
