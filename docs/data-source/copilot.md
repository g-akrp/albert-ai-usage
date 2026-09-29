---
title: Copilot data source
description: How to get real GitHub Copilot usage data, verified live.
date: 2026-09-29
---

# Copilot data source

Verified 2026-09-29 against `gh version 2.101.0`, locally installed.

## The call

```bash
gh api copilot_internal/user
```

Internal/undocumented endpoint (no `docs.github.com` page), but a plain `gh auth login` OAuth token works fine — no special app registration or extra scope needed. `gh`'s own auth handles the token; nothing here reads a credential file directly.

## The gotcha: a stale env var silently wins

If `GITHUB_TOKEN` or `GH_TOKEN` is set in the environment (even to an invalid value), `gh` uses it over the working keyring-stored login and fails with a generic `401 Bad credentials` — not an error naming the env var as the cause. Confirmed live:

```
$ gh api copilot_internal/user
{"message":"Bad credentials","status":"401"}

$ env -u GITHUB_TOKEN -u GH_TOKEN gh api copilot_internal/user
{...real data...}
```

The adapter strips `GITHUB_TOKEN`/`GH_TOKEN` for the `gh` child process only (`Command::env_remove`, `process::call_copilot_user`) — the parent shell's environment is never touched — so it reliably falls back to `gh`'s own stored keyring login instead of failing on a stale var. If it still fails after that, the sanitized error says "gh authentication or API call failed" and the human checks `gh auth status` themselves; the code never guesses further.

## Real response shape (trimmed to relevant fields)

```json
{
  "copilot_plan": "individual",
  "quota_reset_date": "2026-10-01",
  "quota_snapshots": {
    "chat": {"entitlement": 200, "remaining": 200, "credits_used": 0, "unlimited": false, "percent_remaining": 100.0},
    "completions": {"entitlement": 2000, "remaining": 806, "credits_used": 1194, "unlimited": false, "percent_remaining": 40.3},
    "premium_interactions": {"entitlement": 0, "remaining": 0, "credits_used": 0, "unlimited": false, "percent_remaining": 0.0}
  }
}
```

Three quota categories, all the same used/limit shape — not just `premium_interactions`. `entitlement` is the limit, `credits_used` is used; `remaining` is redundant (`entitlement - credits_used`, confirmed to match exactly across all three in the live sample) so only `entitlement`/`credits_used` need reading.

## Multiple accounts

A person can have more than one GitHub account logged into `gh` at once (e.g. a personal account plus a work seat) — confirmed on this machine: `g-akrp` (individual plan) and `2521180709_bblghcp` (business seat), each with genuinely different quota numbers.

`gh api` has no per-call account-selector flag, and only one account is "active" at a time. Instead: `gh auth token --user <account>` reads that specific account's already-stored token without switching gh's global active account or mutating any state — safe to use per-call.

The live path enumerates every account `gh` has stored, by reading `~/.config/gh/hosts.yml` directly (`process::list_gh_accounts` / `parse_gh_accounts`) — no documented `gh` command prints a clean account list; `gh auth status` is human text only. One `CopilotProvider` instance is built per account (`CopilotProvider::for_account`), each reporting independently with id `copilot:<account>`.

**Known limitation:** this is a best-effort parse of `gh`'s own config file format, not a stable public API — could break on a future `gh` version. Also macOS/Linux only; Windows stores this at `%APPDATA%\GitHub CLI\hosts.yml`, not handled yet (no Windows shell exists in V0.1.0 anyway).

## Allowlist — fields this codebase reads

| Field | Type | Notes |
|---|---|---|
| `copilot_plan` | string | for the note |
| `login` | string | public username, not a secret — needed to tell multiple accounts apart in output |
| `quota_snapshots.{chat,completions,premium_interactions}.entitlement` | integer | limit |
| `quota_snapshots.{chat,completions,premium_interactions}.credits_used` | integer | used |

## Never read, dropped by construction

`id` (the opaque numeric account id, distinct from `login`), `analytics_tracking_id`, `enterprise_list`, `organization_login_list`, `organization_list`, `endpoints` (internal API URLs), `is_staff`, `cli_remote_control_enabled`, and any other field not listed above. No field exists for these in the deserialize target, so serde drops them — same compile-time-shape guarantee as the Codex adapter.

## Safety notes

- Live call is off by default (`ALBERT_LIVE_COPILOT=1`, `make run-live-copilot`).
- No credential file is read — `gh` handles its own stored token.
- Non-zero exit, malformed JSON, or a missing `quota_snapshots` key all degrade to `ProviderStatus::Unsupported` with a sanitized reason.
- Nothing from `gh`'s raw stderr is logged verbatim, in case a future `gh` version's error text ever changed to include anything sensitive.
