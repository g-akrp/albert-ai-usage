# Copilot premium interactions: used/quota count

Status: approved (2026-10-08), specs updated; see CHANGES.md
Date: 2026-10-08

## Problem
The Premium Interactions meter for GitHub Copilot (Business and other plans) shows only a percent. Users on a monthly request quota want the absolute numbers, for example `120/300`.

## Proposal
Show used/quota next to the Premium Interactions meter. The data already exists: `gh api copilot_internal/user` returns `quota_snapshots/premium_interactions/entitlement` and `quota_remaining`. Used is `entitlement - quota_remaining`. Quota is `entitlement`. With `token_based_billing` true the numbers are AI credits (1 credit = $0.01), not requests, so the label says credits, for example `6280/10000 credits`. Do not use `credits_used`. See `copilot-premium-count.research.md`. Reset time is a separate idea: [copilot-reset-time.md](copilot-reset-time.md).

## Scope
In: Premium Interactions meter of the `copilot` provider only. Count shown as text on that meter's row in the panel.
Out: Chat and Completions meters, other providers, menu bar icon value, reset time.

## Alternatives considered
- Hard-code a Copilot-only label in the shell: rejected, the provider format should stay data-driven.
- Replace the percent with the count: see open questions.

## Impact
- `docs/specs/data-source/data-source.md`: provider format gets an optional way to map a used count and a limit count on a window.
- `docs/specs/data-source/github-copilot.md`: premium meter maps `entitlement` and `quota_remaining`.
- `docs/specs/menubar/menubar-panel.md`: meter row shows the count.
- Repo code: `ProviderConfig`, `ProviderRunner`, `CardModel`, `CardView`, `Resources/providers/copilot.json` (bump `revision`), README, CoreChecks.

## Decisions (user, 2026-10-08)
1. The percent stays; the count is shown next to it (`63% · 6280/10000 credits`).
2. Accounts where `entitlement` is absent or 0: the meter stays hidden as today.
3. Overage (`overage_count` > 0): show used above quota, for example `10320/10000`. Unverified: no account with overage was available, so whether `entitlement - quota_remaining` still holds is unknown.
4. Unit label: `credits` when `token_based_billing` is true; no unit when it is false (the unit for request-based accounts is unverified).

## Open questions
None.
