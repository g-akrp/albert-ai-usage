# Windows panel

Ring percentages use DirectWrite horizontal and paragraph centering within the ring bounds. Expand/collapse controls use matching vector chevrons centered in their hit targets rather than font glyphs.

Fresh Windows settings enable Claude and Codex only. Copilot and Antigravity remain available in Providers for opt-in use. Existing explicit provider preferences are preserved. Live CLI diagnostics respect the same provider switches.

When a provider explicitly reports unavailable usage and supplies no meters, both expanded and collapsed cards show "Usage unavailable". No percentage pill or chart is synthesized; the tray percentage remains `--`. On the tested Windows device, Claude CLI authentication metadata showed it was logged out. Sign in using `claude auth login`, then refresh; authentication remains owned by Claude's CLI.

The native Direct2D/DirectWrite panel shows provider cards, grouped rings, flat counted quotas, local reset times, relative reset labels, and collapsed percentage pills. Users can refresh, pin, hide, collapse, reorder, enable providers, and choose launch-at-login. Settings are written atomically. The app uses a single instance and a tray popup; demo mode uses separate fixture settings.

The x64 debug panel and Refresh action were observed working. Computer Use was stopped by the user; complete keyboard, focus, scrolling, theme, DPI, Explorer restart, and UI endurance checks remain pending. A full custom UI Automation tree is outside current scope.
