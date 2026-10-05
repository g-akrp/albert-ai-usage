# GitHub Copilot

File: `Resources/providers/copilot.json`. Icon `GHC` (`GH1`, `GH2`, … with several accounts), color `8957E5`. Refresh every 5 minutes, timeout 20 s.

## Accounts

Runs `gh auth status --json hosts` and takes every account on `github.com` whose state is `success`. The provider runs once per account, without switching the active account. Each account is its own block in the panel and its own run id (`copilot:<login>`).

## Source

For each account: `gh auth token --user <account>` sets `GH_TOKEN` inside that child process only, then `gh api copilot_internal/user`. The token is never stored. Requires `quota_snapshots` in the answer.

## Mapping

- Plan: `copilot_plan`.
- Three meters from `quota_snapshots`, each with one window (`current`); used percent is `100 - percent_remaining`. No reset time is shown:
  - Chat (`chat`)
  - Completions (`completions`)
  - Premium Interactions (`premium_interactions`)
- A meter is skipped unless `has_quota` is true and `unlimited` is false. Copilot Free has no premium requests, reported as 0% remaining, so the meter is hidden.

## Icon value

No session or weekly window exists, so the value is Premium Interactions when present, otherwise the highest of the remaining meters. Color follows the highest meter.
