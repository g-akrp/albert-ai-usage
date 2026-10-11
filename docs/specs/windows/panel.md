# Windows panel

Ring percentages use DirectWrite horizontal and paragraph centering within the ring bounds. Expand/collapse controls use matching vector chevrons centered in their hit targets rather than font glyphs.

Fresh Windows settings enable Claude and Codex only. Copilot and Antigravity remain available in Providers for opt-in use. Existing explicit provider preferences are preserved. Live CLI diagnostics respect the same provider switches.

When a provider explicitly reports unavailable usage and supplies no meters, both expanded and collapsed cards show "Usage unavailable". No percentage pill or chart is synthesized; the tray percentage remains `--`. On the tested Windows device, Claude CLI authentication metadata showed it was logged out. Sign in using `claude auth login`, then refresh; authentication remains owned by Claude's CLI.

The native Direct2D/DirectWrite panel shows provider cards, grouped rings, flat counted quotas, local reset times, relative reset labels, and collapsed percentage pills. Users can refresh, pin, hide, collapse, reorder, enable providers, and choose launch-at-login. Settings are written atomically. The app uses a single instance and a tray popup; demo mode uses separate fixture settings.

The x64 debug panel and Refresh action were observed working. Computer Use was stopped by the user; complete keyboard, focus, scrolling, theme, DPI, Explorer restart, and UI endurance checks remain pending. A full custom UI Automation tree is outside current scope.

## Motion and feedback

- Actions stay in the flyout: Pin, Hide, collapse, move, Retry, and Launch at Login re-lay out the open flyout in place, anchored to the taskbar, with keyboard focus kept on the same control. Open Providers Folder and Quit close it; Escape and clicking outside still close it at once.
- Opening slides the flyout 10 DIP in from the taskbar side and fades its content in, over 180 ms with an ease-out curve. Closing is instant.
- Rings sweep from the value last shown to the new one over 300 ms; a ring seen for the first time sweeps from 0. Unchanged rings do not move. New results that arrive while the flyout is open sweep too.
- While a provider runs, its card shows a small spinner after the title, and the footer reads `Refreshing…` (not clickable) instead of Refresh Now.
- When results arrive while the flyout is open, the `Updated …` line turns the accent color and fades back over 1 s.
- A failed card shows a Retry button that runs that provider now, and a muted line with the automatic backoff: `Retrying in 4m`, or `Retrying soon` when it is due. Both are hidden while the provider is running.
- Reduced motion: when Windows Settings › Accessibility › Visual effects › Animation effects is off, nothing slides, fades, or sweeps, and the spinner is a still dot.
- Cost: a 16 ms animation timer runs only while the flyout is open and something moves or spins. Measured on the x64 release demo: about 16 ms of CPU over 5 s once settled, 8 MB private working set.
