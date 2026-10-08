# Mission 0001: Copilot premium count and reset time

## Goal
CHANGES.md 2026-10-08 "Copilot premium count and reset time".
Specs covered (read-only, nobody edits specs/ or CHANGES.md):
- docs/specs/data-source/data-source.md
- docs/specs/data-source/github-copilot.md
- docs/specs/menubar/menubar-panel.md

Branch: mission-0001-copilot-count-reset, base 1ca88da.
Acceptance: `swift build` and `swift run CoreChecks` pass; Reviewer confirms no specs/ or CHANGES.md edits.

## Plan
CoreChecks first (failing), then Core, then Model+UI, then provider JSON, then docs. Strict order: T1, T2, T3, T4, T5. One Coder, one task at a time. Tester verifies each task, Reviewer reviews at the end.

## Task list
| ID | Task | File area | Status |
| - | - | - | - |
| T1 | Failing CoreChecks | Sources/CoreChecks | done (16e4292), Gauge PASS |
| T2 | Core: resetsAt.from root + window count | Sources/AIUsageCore (ProviderConfig, Report, ProviderRunner) | done (4175fe5), Gauge PASS |
| T3 | Model+UI: row count text | CardModel, MenuModel, Sources/AIUsage/CardView | done (074aacc), verify pending |
| T4 | copilot.json revision 5 | Resources/providers/copilot.json | done (68cbc35), match held |
| T5 | Docs: resetsAt.from and count | example/AGENTS.md, README.md | done (6c3da5d), README only |

### Briefs (each: file, one change, acceptance, constraints)
T1. File: Sources/CoreChecks (ModelChecks.swift, RunnerChecks.swift). Change: add checks, read specs first. Cover: (a) resetsAt.from root reads reset from the top-level answer; (b) count parse: limit 0 or missing, remaining missing, conditional unit matches only when condition true, overage unclamped (used = limit - remaining); (c) row pill text "63% · 6280/10000 credits" and "63% · 120/300" (no unit); (d) collapsed pills percent-only. Acceptance: `swift build` passes; `swift run CoreChecks` fails only on the new checks (compile errors on missing types are acceptable only if stubs are not added; prefer checks that compile and fail). Constraints: no edits outside Sources/CoreChecks; no specs/ or CHANGES.md edits.
T2. Files: ProviderConfig.swift, Report.swift, ProviderRunner.swift. Change: support resetsAt.from "root" and window count (limit, remaining, optional conditional unit); used = limit - remaining, not clamped. Acceptance: T1 parse/reset checks pass; build passes. Constraints: no UI files; no specs/CHANGES edits; do not change CoreChecks expectations.
T3. Files: CardModel.swift, MenuModel.swift, Sources/AIUsage/CardView.swift. Change: row pill text "63% · 6280/10000 credits" / "63% · 120/300"; pill sizing and drawing fit the longer text; collapsed pills stay percent-only. Acceptance: all CoreChecks pass; build passes. Constraints: no spec edits.
T4. File: Resources/providers/copilot.json. Change: revision 4 to 5; resetsAt {from root, path /quota_reset_date_utc, iso8601} on all 3 meters; premium count limit /entitlement, remaining /quota_remaining, unit "credits" only when /token_based_billing is true; premium match has_quota true and unlimited false. Acceptance: JSON valid; build and CoreChecks pass. Constraints: one file; no spec edits.
T5. Files: example/AGENTS.md, README.md. Change: document resetsAt.from and count in provider-format sections. Acceptance: text matches spec data-source.md wording. Constraints: docs only; no specs/CHANGES edits.

## Per-task result
T1: Anvil added countChecks() in ModelChecks.swift, called from cardChecks. Build passes; CoreChecks 6 FAILED, all new: resetFromRoot, countCredits, countUnitOnlyWhenConditionTrue, countUnitConditionMissing, countNoUnitSpec, countOverageUnclamped. Guard checks already passing. Commit 16e4292. Gauge PASS.

T2: Anvil changed ProviderConfig.swift (resetsAt.from root, CountSpec parse) and Report.swift (WindowCount, WindowReport.count, Mapper root-scope reset and count; used=limit-remaining unclamped; none when limit 0/missing or remaining missing; unit only when match holds). ProviderRunner.swift unchanged, not needed. Build passes. resetFromRoot passes; 5 pill-text checks still fail (T3 scope): countCredits, countUnitOnlyWhenConditionTrue, countUnitConditionMissing, countNoUnitSpec, countOverageUnclamped. Commit 4175fe5. Gauge PASS.

T3: Anvil changed CardModel.swift only. CardRow.percentText now includes count; new CardRow.percentOnly feeds collapsed pills; merged Copilot chart passes window count through. CardView and MenuModel unchanged: Anvil says widths derive from measured percentText. Build passes, CoreChecks all pass. Not visually checked in running app. Commit 074aacc.

T4: Anvil, copilot.json: revision 5, resetsAt root on 3 meters, premium count (credits unit when /token_based_billing true). Premium match kept ORIGINAL (/entitlement notEquals 0, unlimited false) per human hold, because spec github-copilot.md says has_quota true and conflicts with existing check copilotShowsExhaustedQuota. Build ok, CoreChecks ALL PASS. Commit 68cbc35.

T5: Anvil, README.md only: resetsAt.from and count bullets added to additions list in 'Add or change a provider'. example/AGENTS.md skipped on purpose: Maestri-owned base format doc that lists no project extensions. Commit 6c3da5d.

## Final summary
(pending)

## Open issues
- OPEN: premium match deviates from spec github-copilot.md line 21 (has_quota true) pending Project Lead decision relayed by human. Check copilotShowsExhaustedQuota (ModelChecks.swift) expects the old behaviour.
- example/AGENTS.md not updated (see T5). Brief said add there; skipped, needs human OK.
- Disk nearly full (2.7Gi free); scratch worktree builds fail.
- Crew: Anvil (Coder), Gauge (Tester), Lens (Reviewer).
- Spec edits: task brief says nobody edits specs/, so no repo-spec tasks.
