# Codex

File: `Resources/providers/codex.json`. Icon `CDX`, color `10A37F`. Refresh every 5 minutes, timeout 20 s.

## Source

Runs `codex app-server --stdio`, sends `initialize`, then `account/rateLimits/read`, and captures the result.

## Mapping

- Plan: `rateLimits.planType`.
- `access`: `ordinaryUsageAllowed`. False shows `Usage limit reached`.
- One meter from `rateLimits` (id `limitId`, label `limitName`, fallback `Codex`) with two windows, used percent from `usedPercent`, reset time from `resetsAt` (epoch seconds), length from `windowDurationMins`:
  - Session: `primary`.
  - Weekly: `secondary`.

## Icon value

Session first, then weekly. Color follows the higher of the two.
