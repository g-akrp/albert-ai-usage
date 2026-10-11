# AI Usage

## Windows preview

The Windows implementation lives in `windows/`: native Rust/Win32 tray UI, Direct2D/DirectWrite rendering, shared JSON provider definitions, and a Windows Copilot adapter. Portable x64 and ARM64 builds are available locally in `windows/dist/`. Extract the matching ZIP and run `ai-usage.exe`; use `--demo` for fixture data with separate settings. Provider tools (`claude`, `codex`, `gh`, `agy`) must be installed and logged in through their own CLIs.

Build from PowerShell with `windows/scripts/build.ps1 -Arch all`, then package with `windows/scripts/package.ps1 -Arch all -PortableOnly`. Settings live under `%APPDATA%\ai-usage`; launch-at-login is optional. Updates are manual: quit, replace the executable, restart. Builds are unsigned. ARM64 runtime, full DPI/keyboard/tray behavior, and release memory endurance validation remain pending. MSI work is deferred by user choice; WiX terms have not been accepted.

See [Windows verification and distribution](docs/specs/windows/release.md).

A small native macOS menu bar app that shows plan usage for AI coding agents: Claude Code, Codex, GitHub Copilot, and Antigravity. It replaces the Rust core and SwiftBar plugin that used to live in this repository (still in git history).

- The menu bar icon is a two-row pixel label: the provider's short name in its brand color on top, its headline percent below (session limit, else weekly, else premium interactions, else the highest; the color follows the highest used percent) in green, orange (from 70%), or red (from 90%). A provider that fails shows a red `ERR`.
- The icon always shows one pinned provider or limit. The first provider is pinned until you pin another; hover a card and click **Pin** to move it. There is no cycle mode.
- The menu shows one card per provider: concentric rings (session outermost, then weekly) on the left, each limit with its reset date and time left (`in 2 minutes`) on the right.
- A provider with several limits (Antigravity's model groups such as Gemini Models, Copilot's Chat and Premium Interactions) has its own pin (Antigravity pins only a model group). Click one to pin just that limit; the icon's top row then shows the first three letters of its name, for example `GEM`, in the provider's color.
- Hover a card and click **Hide** to hide it, for example one Copilot account. The provider keeps running; **Hidden Cards** in the menu shows it again.
- **Providers** in the menu turns monitoring of each provider on or off. A provider that is off never runs and is hidden from the menu bar and the menu; turning it back on runs it right away. The choice is kept across launches.
- GitHub Copilot shows every account `gh` is logged in to (`gh auth status`), without switching the active account. With more than one account the icon labels are `GH1`, `GH2`, … in the order `gh` lists them.
- The menu lists every provider with its plan and each limit window, with reset times in your local time zone, and the reason when a provider fails.
- Each provider refreshes on its own interval (5 minutes; Antigravity 10 minutes), at most two at a time. A failing provider retries after 1, 2, 4, … minutes, at most every 30. **Refresh Now** (⌘R) runs all of them.
- No Dock icon. AppKit only, no third-party packages.

Requires macOS 13 or later, and the provider CLIs you want to see: `claude`, `codex`, `gh` (logged in, with Copilot), `agy`.

## Install and update (release DMG)

1. Download `AIUsage-<version>.dmg` from the GitHub release (the repository is private, so you need access).
2. Open it and drag **AI Usage** onto **Applications**.
3. First launch: macOS blocks the app because it is signed ad hoc, not with a paid Apple Developer ID. Open it once, then go to System Settings › Privacy & Security and choose **Open Anyway**.
4. Optional: choose **Launch at Login** in the menu.

To update: quit the app (menu › Quit), install the new DMG the same way (replace the old app), and open it. The installed version is shown at the bottom of the menu.

To check a download: `shasum -a 256 -c AIUsage-<version>.dmg.sha256` in the download folder.

Versions before 1.3.0 were called Albert AI Usage (`AlbertAIUsage.app`). Quit it and delete it after installing AI Usage; your pin and provider switches carry over. Turn **Launch at Login** on again, because it belongs to the old app.

If you used the SwiftBar plugin, remove its symlink `albert-usage.30s.sh` from SwiftBar's plugin folder. The app reads the plugin's old pin (`~/.config/albert-ai-usage/pinned`) once on first launch.

## Providers

A provider is a JSON file, not code: which program to run and how to read its answer. The format is the Maestri Agent Usage format, documented in [example/AGENTS.md](example/AGENTS.md), with these additions:

- `iconLabel`: the icon's top row. Default: the first three letters of the id.
- `iconColor`: `"RRGGBB"`. Default: gray.
- `"as": "remainingPercent"` for `used`: 0 to 100 left.
- `match` on a meter or window: the same predicates as `expect`; a value where one fails is skipped. Also `notEquals`. Copilot uses it to skip quotas that do not apply: `unlimited` true, or an `entitlement` of 0 (Copilot Free has no premium requests, reported as 0% remaining). A quota with `has_quota` false but an entitlement (a business seat that used all its premium requests) is shown as 100% used.
- `accounts`: run the provider once per account. Its `source` lists the accounts, `each` points to the list, `id` to each account's name, and an optional `match` filters them. `${account}` in the provider's `args` and `env` is replaced by the name. Copilot lists `gh auth status --json hosts` and runs `gh api` with `GH_TOKEN` set from `gh auth token --user <account>` inside that child process only; the token is never stored.

The app ships `Resources/providers/*.json`. To add a provider or change a shipped one, choose **Open Providers Folder…** and put a `<id>.json` file in `~/.config/ai-usage/providers/`. A file there with the same id replaces the shipped one. Files are reread on **Refresh Now**; a file that does not load is listed in the menu with the reason.

How providers run:

- Launched directly, without a shell, from an empty temporary folder, in their own process group. Stderr is discarded. On timeout the whole group is killed.
- Minimal environment: `PATH`, `HOME`, `TERM=dumb`, `NO_COLOR=1`, `LANG`, and when set: `USER`, `LOGNAME`, `LC_ALL`, `TMPDIR`, `SHELL`, `SSH_AUTH_SOCK`, `__CF_USER_TEXT_ENCODING`, and the `HTTP(S)_PROXY`, `NO_PROXY`, `ALL_PROXY` variables (both cases). `GITHUB_TOKEN` and `GH_TOKEN` are never passed, because a stale one shadows a working `gh` login. Add anything else with `env` in the provider file.
- `PATH` and the proxy variables come from your login shell (`$SHELL -l -i`, read once at launch), followed by `~/.local/bin`, `~/bin`, `/opt/homebrew/bin`, `/usr/local/bin`, and the system folders. An app opened from Finder does not otherwise see your shell's PATH.
- Output limits from the format are enforced (`maxOutputBytes`, `maxLineBytes`, `maxTotalBytes`).

## Making a release

```
scripts/release.sh 1.1.0            # build locally: version, checks, DMG, commit, tag
scripts/release.sh 1.1.0 --publish  # also push main and the tag, and create the GitHub release
```

The script runs only on a `main` without uncommitted changes to tracked files. It sets `CFBundleShortVersionString` (and increments `CFBundleVersion`) in `Resources/Info.plist` and `AppVersion.current`, runs `swift run CoreChecks`, builds the app, and writes `build/AIUsage-<version>.dmg` with a `.sha256` file. If anything fails before the release commit, the version edits are undone. Publishing needs the `gh` CLI logged in and an `origin` remote.

## Build from source

```
scripts/build-app.sh
open ~/Applications/"AI Usage.app"
```

The script builds the app, wraps it with the provider files in `AI Usage.app`, signs it ad hoc, and copies it to `~/Applications`. Only the Xcode Command Line Tools are needed.

The app icon (`Resources/AppIcon.icns`) is drawn by `scripts/make-icon.swift` in the same pixel style. To change it, edit that script and run `swift scripts/make-icon.swift`.

## Checks

```
swift run CoreChecks          # offline checks; PASS/FAIL per check, non-zero exit on any failure
swift run CoreChecks --live   # runs the shipped providers for real and prints what the menu would show
```

The checks live in an executable target, not XCTest, because XCTest and Swift Testing do not build with the Command Line Tools alone.

## Layout

| Path | What |
|------|------|
| `Sources/AIUsageCore/` | Provider format parsing, mapping, process runner, environment, pixel font, menu model |
| `Sources/AIUsage/` | AppKit shell: `StatusController` (icon, menu), `Poller` (schedule) |
| `Sources/CoreChecks/` | Check runner |
| `Resources/` | `Info.plist`, `AppIcon.icns`, shipped `providers/` |
| `example/` | Maestri's provider format guide and its shipped examples |
| `docs/data-source/` | Research notes on each provider's usage source |

## Resource use

Measured on macOS 26.7 one minute after launch, with all four providers refreshed: physical footprint 11 MB, CPU 0.0%, release binary 360 KB. Provider CLIs run as short-lived child processes and are not part of that number.

## Not yet verified

- Launch at Login (`SMAppService.mainApp`) for an ad hoc signed app.
- Installing from the DMG on another Mac (Gatekeeper's **Open Anyway** step).
