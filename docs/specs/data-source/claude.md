# Claude Code

File: `Resources/providers/claude.json`. Icon `CLD`, color `D97757`. Refresh every 5 minutes, timeout 90 s.

## Source

Runs `claude -p --input-format stream-json --output-format stream-json --verbose --no-session-persistence`, writes a `get_usage` control request, and waits for the `control_response` with request id `ai-usage`.

## Mapping

- Plan: `subscription_type`.
- `available`: `rate_limits_available`. False means plan limits do not apply.
- One meter `plan` with two windows, each `utilization` as used percent, reset time from `resets_at` (ISO 8601):
  - Session: `five_hour`, 5 hours.
  - Weekly: `seven_day`, 7 days.

## Icon value

Session first, then weekly (see [menu bar icon](../menubar/menubar-icon.md)). Color follows the higher of the two.
