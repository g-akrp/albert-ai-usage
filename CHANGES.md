# Spec changes

Newest first. In-flight policy: `finish-then-follow-up` (default) or `replan-now`.

## 2026-10-08: Copilot premium count and reset time

- **Spec files:** `docs/specs/data-source/data-source.md`, `docs/specs/data-source/github-copilot.md`, `docs/specs/menubar/menubar-panel.md`
- **Summary:**
  - Provider format: `resetsAt.from: "root"` reads a reset path from the top-level answer; a window may have `count` (`limit`, `remaining`, optional conditional `unit`). Used = `limit - remaining`, not clamped.
  - Copilot: reset time `/quota_reset_date_utc` on every meter; Premium Interactions count `entitlement` / `quota_remaining`, unit `credits` only when `token_based_billing` is true.
  - Panel: the pill shows `63% · 6280/10000 credits`; collapsed pills show the percent only.
- **Repos affected:** albert-ai-usage (one repo, no roadmap file)
- **In-flight policy:** finish-then-follow-up
- **Contract decisions:** the format additions above are the contract. `credits_used` is not used. Overage is shown unclamped (unverified against a live overage account). Source: `docs/ideas/copilot-premium-count.md`, `docs/ideas/copilot-reset-time.md`, `docs/ideas/copilot-premium-count.research.md`.
- **Decision (same day):** Premium Interactions keeps the `entitlement != 0` rule, not `has_quota`, so an exhausted Business quota still shows its meter (check `copilotShowsExhaustedQuota`). The `github-copilot.md` line was corrected; Chat and Completions keep `has_quota`.
- **Clarification (same day):** `resetsAt.from: "root"` reads the `root` scope, which is the value at the map's `root` (the whole answer when unset). Matches the shipped Report.swift. `example/AGENTS.md` (Maestri-owned base format) is not updated; extensions are documented in README.
- **Follow-up for the Repo Lead:** bump `revision` in `Resources/providers/copilot.json`; update `example/AGENTS.md` and README if they describe the format; add CoreChecks first.
