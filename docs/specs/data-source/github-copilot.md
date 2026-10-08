# GitHub Copilot

File: `Resources/providers/copilot.json`. Icon `GHC` (`GH1`, `GH2`, … with several accounts), color `8957E5`. Refresh every 5 minutes, timeout 20 s.

## Accounts

Runs `gh auth status --json hosts` and takes every account on `github.com` whose state is `success`. The provider runs once per account, without switching the active account. Each account is its own block in the panel and its own run id (`copilot:<login>`).

## Source

For each account: `gh auth token --user <account>` sets `GH_TOKEN` inside that child process only, then `gh api copilot_internal/user`. The token is never stored. Requires `quota_snapshots` in the answer.

## Mapping

- Plan: `copilot_plan`.
- Three meters from `quota_snapshots`, each with one window (`current`); used percent is `100 - percent_remaining`. Reset time is `/quota_reset_date_utc` (ISO 8601, read from the root; the per-meter `quota_reset_at` is always 0), the same on every meter:
  - Chat (`chat`)
  - Completions (`completions`)
  - Premium Interactions (`premium_interactions`)
- Premium Interactions also has a count: limit `entitlement`, remaining `quota_remaining`, so used is `entitlement - quota_remaining`. Unit `credits` when `token_based_billing` is true, no unit when it is false. Chat and Completions have no count. (Internal endpoint: `token_based_billing` true means the numbers are AI credits, 1 credit = $0.01. `credits_used` is not used. See `docs/ideas/copilot-premium-count.research.md`.)
- A meter is skipped unless `has_quota` is true and `unlimited` is false. Copilot Free has no premium requests, reported as 0% remaining, so the meter is hidden.

## Icon value

No session or weekly window exists, so the value is Premium Interactions when present, otherwise the highest of the remaining meters. Color follows the highest meter.
