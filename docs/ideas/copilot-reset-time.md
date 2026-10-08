# Copilot reset time

Status: implemented (2026-10-08, merge bb99247); see CHANGES.md
Date: 2026-10-08

## Problem
Copilot meters show no reset time (`docs/specs/data-source/github-copilot.md`: "No reset time is shown"), while Claude Code and Codex show one. The per-meter `quota_reset_at` is 0 on every snapshot, so it cannot be used.

## Proposal
Map the top-level `/quota_reset_date_utc` (for example `2026-11-01T00:00:00.000Z`) as the reset time of the Copilot meters, shown like other providers' reset times (Today/Tomorrow and weekday).

## Scope
In: the `copilot` provider's Premium Interactions meter, and Chat and Completions when they are shown.
Out: other providers, icon value.

## Alternatives considered
- Per-meter `quota_reset_at`: rejected, always 0.

## Impact
- `docs/specs/data-source/github-copilot.md`: mapping gains a reset time.
- `Resources/providers/copilot.json` (bump `revision`). The provider format must allow a reset path outside the selected meter (top-level pointer); check `data-source.md`, which may need a change.
- Related: [copilot-premium-count.md](copilot-premium-count.md).

## Open questions
1. Does the provider format already allow a reset path outside the selected object? If not, this needs a format change.
2. Source: research notes in `copilot-premium-count.research.md`. The field is internal and undocumented.
