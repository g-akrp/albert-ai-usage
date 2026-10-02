# Albert AI Usage

A small native macOS menu bar app that shows plan usage for AI coding agents: Claude Code, Codex, GitHub Copilot, and Antigravity. It replaces the Rust core and SwiftBar plugin that used to live in this repository (still in git history).

- The menu bar icon is a two-row pixel label: the provider's short name in its brand color on top, its highest used percent below in green, orange (from 70%), or red (from 90%). A provider that fails shows a red `ERR`.
- Nothing pinned: the icon cycles through every provider every 5 seconds. Click a provider in the menu to pin it; choose **Cycle All Providers** to unpin.
- A provider with several limits (Antigravity's model groups such as Gemini Models, Copilot's Chat and Premium Interactions) lists each with a ☆. Click one to pin just that limit; the icon's top row then shows the first three letters of its name, for example `GEM`, in the provider's color.
- GitHub Copilot shows every account `gh` is logged in to (`gh auth status`), without switching the active account. With more than one account the icon labels are `GH1`, `GH2`, … in the order `gh` lists them.
- The menu lists every provider with its plan and each limit window, with reset times in your local time zone, and the reason when a provider fails.
- Each provider refreshes on its own interval (5 minutes; Antigravity 10 minutes), at most two at a time. A failing provider retries after 1, 2, 4, … minutes, at most every 30. **Refresh Now** (⌘R) runs all of them.
- No Dock icon. AppKit only, no third-party packages.

Requires macOS 13 or later, and the provider CLIs you want to see: `claude`, `codex`, `gh` (logged in, with Copilot), `agy`.

## Install and update (release DMG)

1. Download `AlbertAIUsage-<version>.dmg` from the GitHub release (the repository is private, so you need access).
2. Open it and drag **AlbertAIUsage** onto **Applications**.
3. First launch: macOS blocks the app because it is signed ad hoc, not with a paid Apple Developer ID. Open it once, then go to System Settings › Privacy & Security and choose **Open Anyway**.
4. Optional: choose **Launch at Login** in the menu.

To update: quit the app (menu › Quit), install the new DMG the same way (replace the old app), and open it. The installed version is shown at the bottom of the menu.

To check a download: `shasum -a 256 -c AlbertAIUsage-<version>.dmg.sha256` in the download folder.

If you used the SwiftBar plugin, remove its symlink `albert-usage.30s.sh` from SwiftBar's plugin folder. The app reads the plugin's old pin (`~/.config/albert-ai-usage/pinned`) once on first launch.

## Providers

A provider is a JSON file, not code: which program to run and how to read its answer. The format is the Maestri Agent Usage format, documented in [example/AGENTS.md](example/AGENTS.md), with these additions:

- `iconLabel`: the icon's top row. Default: the first three letters of the id.
- `iconColor`: `"RRGGBB"`. Default: gray.
- `"as": "remainingPercent"` for `used`: 0 to 100 left.
- `match` on a meter or window: the same predicates as `expect`; a value where one fails is skipped. Copilot uses it to skip quotas that do not apply: `has_quota` false (Copilot Free has no premium requests, reported as 0% remaining) or `unlimited` true.
- `accounts`: run the provider once per account. Its `source` lists the accounts, `each` points to the list, `id` to each account's name, and an optional `match` filters them. `${account}` in the provider's `args` and `env` is replaced by the name. Copilot lists `gh auth status --json hosts` and runs `gh api` with `GH_TOKEN` set from `gh auth token --user <account>` inside that child process only; the token is never stored.

The app ships `Resources/providers/*.json`. To add a provider or change a shipped one, choose **Open Providers Folder…** and put a `<id>.json` file in `~/.config/albert-ai-usage/providers/`. A file there with the same id replaces the shipped one. Files are reread on **Refresh Now**; a file that does not load is listed in the menu with the reason.

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

The script runs only on a `main` without uncommitted changes to tracked files. It sets `CFBundleShortVersionString` (and increments `CFBundleVersion`) in `Resources/Info.plist` and `AppVersion.current`, runs `swift run CoreChecks`, builds the app, and writes `build/AlbertAIUsage-<version>.dmg` with a `.sha256` file. If anything fails before the release commit, the version edits are undone. Publishing needs the `gh` CLI logged in and an `origin` remote.

## Build from source

```
scripts/build-app.sh
open ~/Applications/AlbertAIUsage.app
```

The script builds the app, wraps it with the provider files in `AlbertAIUsage.app`, signs it ad hoc, and copies it to `~/Applications`. Only the Xcode Command Line Tools are needed.

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
| `Sources/AlbertUsageCore/` | Provider format parsing, mapping, process runner, environment, pixel font, menu model |
| `Sources/AlbertAIUsage/` | AppKit shell: `StatusController` (icon, menu), `Poller` (schedule) |
| `Sources/CoreChecks/` | Check runner |
| `Resources/` | `Info.plist`, `AppIcon.icns`, shipped `providers/` |
| `example/` | Maestri's provider format guide and its shipped examples |
| `docs/data-source/` | Research notes on each provider's usage source |

## Resource use

Measured on macOS 26.7 one minute after launch, with all four providers refreshed: physical footprint 11 MB, CPU 0.0%, release binary 360 KB. Provider CLIs run as short-lived child processes and are not part of that number.

## Not yet verified

- Launch at Login (`SMAppService.mainApp`) for an ad hoc signed app.
- Installing from the DMG on another Mac (Gatekeeper's **Open Anyway** step).
