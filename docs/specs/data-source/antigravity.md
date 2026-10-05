# Antigravity

File: `Resources/providers/antigravity.json`. Icon `AGY`, color `4285F4`. Refresh every 10 minutes, timeout 90 s.

## Source

Runs `agy -p /usage --output-format json`. Requires `status` equal to `SUCCESS` and `command.data.groups`.

## Mapping

- One meter per model group (`name`, for example Gemini Models, Claude and GPT models), from `command.data.groups`.
- One window per bucket: label from the bucket name, used percent from `remaining_fraction` (as remaining), reset time from `reset_time` (ISO 8601), length from `window` (`five_hour`, `daily`, `weekly`).
- Every group is pinnable on its own.

## Icon value

The highest session (five-hour) percent across groups, otherwise the highest weekly, otherwise the highest overall. Color follows the highest overall. Pin a group to see that group alone.
