---
title: Codex data source
description: How to get real Codex usage data, verified against a live app-server session.
---

# Codex data source

Verified 2026-09-29 against `codex-cli 0.158.0`, locally installed. Implementation: `core/src/process.rs` (transport) and `core/src/providers/codex.rs` (parsing). Live path is opt-in only (`ALBERT_LIVE_CODEX=1`, `make run-live`) — the default build never spawns `codex`.

## How to get the schema yourself

Don't guess field names from docs or search results — the CLI generates its own authoritative schema:

```bash
codex app-server generate-json-schema --experimental --out <dir>
```

This writes one JSON Schema file per message type, including `v2/GetAccountRateLimitsResponse.json` and `v2/NullableGetAccountRateLimitsParams.json`. That's how every field name and type below was confirmed, not research summaries.

## Transport

Spawn `codex app-server --listen stdio://` (that's the default; `--listen` can be omitted). Communicate over its stdin/stdout with newline-delimited JSON-RPC. No network port, no socket file, no auth-state file access.

## The handshake

`initialize` requires `params.clientInfo`, or the server ignores the call silently:

```json
{"id":1,"method":"initialize","params":{"clientInfo":{"name":"albert-ai-usage","version":"0.1.0"}}}
```

Wait roughly 500ms after sending before the next request — shorter delays observed empty/incomplete responses during development.

## The rate-limits call

```json
{"id":2,"method":"account/rateLimits/read","params":null}
```

## The gotcha: unsolicited notifications

Between responses, the server pushes notifications with no `id` field, e.g.:

```json
{"method":"remoteControl/status/changed","params":{...},"emittedAtMs":...}
{"method":"account/updated","params":{"authMode":"chatgpt","planType":"plus"},"emittedAtMs":...}
```

**Do not assume the next line on stdout is the response you're waiting for.** Read lines in a loop and only accept one whose `"id"` matches the request you sent; skip everything else. See `process::recv_response_for_id`.

## Real response shape

```json
{
  "id": 2,
  "result": {
    "ordinaryUsageAllowed": true,
    "rateLimits": {
      "primary": {"usedPercent": 46, "windowDurationMins": 300, "resetsAt": 1790680997},
      "secondary": {"usedPercent": 45, "windowDurationMins": 10080, "resetsAt": 1791075882},
      "planType": "plus",
      "limitId": "codex", "limitName": null, "normalModelSlug": null,
      "credits": {"hasCredits": false, "unlimited": false, "balance": "0"},
      "individualLimit": null, "spendControlReached": false, "rateLimitReachedType": null
    },
    "rateLimitsByLimitId": {"codex": { "...same shape as rateLimits..." }},
    "rateLimitResetCredits": {
      "availableCount": 1,
      "credits": [{"id": "...", "resetType": "codexRateLimits", "status": "available", "grantedAt": 0, "expiresAt": 0, "title": "...", "description": "..."}]
    },
    "accountId": "...",
    "rateLimitUpsell": null
  }
}
```

`ordinaryUsageAllowed` and `rateLimitResetCredits` sit at the top level of `result`, siblings of `rateLimits` — not nested inside it.

## Allowlist — fields this codebase reads

| Field | Type | Notes |
|---|---|---|
| `rateLimits.primary.usedPercent` | integer | session window |
| `rateLimits.primary.windowDurationMins` | integer, nullable | |
| `rateLimits.primary.resetsAt` | integer (epoch s), nullable | |
| `rateLimits.secondary.*` | same shape as `primary` | weekly window |
| `rateLimits.planType` | string, nullable | |
| `ordinaryUsageAllowed` | bool, nullable | top level, not under `rateLimits` |
| `rateLimitResetCredits.availableCount` | integer | top level |

## Never read, dropped by construction

`accountId`, `credits` (balance), `rateLimitUpsell`, `rateLimitsByLimitId`, `individualLimit`, `limitId`/`limitName`/`normalModelSlug`, `spendControlReached`, `rateLimitReachedType`, `rateLimitResetCredits.credits` (the detail array), and any field not listed above. The Rust deserialize target (`RawResponse`/`RawRateLimits`/`RawWindow`/`RawResetCredits` in `codex.rs`) simply has no field for these — serde drops unknown fields by default, so nothing here is a runtime filter that could be bypassed; it's a compile-time shape.

## Safety notes

- Live call is off by default (`ALBERT_LIVE_CODEX=1` env var, gated in `CodexProvider::fetch_usage`).
- No auth-state file (`~/.codex/auth.json` or similar) is ever opened.
- Malformed/unexpected response, timeout, or spawn failure all degrade to `ProviderStatus::Unsupported` with a sanitized reason — never a panic, never CLI-text-scraping fallback.
- Bounded: 5s read timeout per line, max 20 notification lines skipped before giving up.
