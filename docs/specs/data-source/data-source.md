# Data source

Usage comes from each provider's own CLI and its existing login. The app never reads or stores credentials, and never passes `GITHUB_TOKEN` or `GH_TOKEN` to a provider (a stale one shadows a working `gh` login).

Providers:

- [Claude Code](claude.md)
- [Codex](codex.md)
- [GitHub Copilot](github-copilot.md)
- [Antigravity](antigravity.md)

## Providers are data

A provider is a JSON file: which program to run and how to map its answer to limits. Format: the Maestri Agent Usage format in `example/AGENTS.md`, plus `iconLabel`, `iconColor`, `"as": "remainingPercent"`, `match` on meters and windows, and `accounts` (see README).

- Shipped files: `Resources/providers/*.json`.
- User files: `~/.config/ai-usage/providers/<id>.json`. Same id replaces the shipped one. Reread on Refresh Now. A file that does not load is listed in the panel with the reason.
- Adding or fixing a provider means editing JSON, not Swift.

## What a run produces

A report: optional plan, optional flags (`available`, `access`), and meters. A meter has windows with used percent, reset time, and length.

## Running

- Launched directly without a shell, from an empty temporary folder, in its own process group. Stderr is discarded. On timeout the whole group is killed.
- Minimal environment: `PATH`, `HOME`, `TERM=dumb`, `NO_COLOR=1`, `LANG`, and when set a short list of others (proxy variables, `TMPDIR`, `SSH_AUTH_SOCK`, …). `PATH` and proxy variables come from the login shell, read once at launch, then `~/.local/bin`, `~/bin`, `/opt/homebrew/bin`, `/usr/local/bin`, system folders.
- Output limits are enforced: `maxOutputBytes`, `maxLineBytes`, `maxTotalBytes`.

## Schedule

- A 30 s timer starts every provider that is due, at most two at a time.
- Interval per provider: 5 minutes (Antigravity 10 minutes).
- After a failure the retry delay is 1, 2, 4, … minutes, at most 30. A success resets it.
- Refresh Now runs all providers at once.
- A provider that is off never runs.

## Required CLIs

`claude`, `codex`, `gh` (logged in, with Copilot), `agy`.
