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
| T1 | Failing CoreChecks | Sources/CoreChecks | todo |
| T2 | Core: resetsAt.from root + window count | Sources/AIUsageCore (ProviderConfig, Report, ProviderRunner) | todo |
| T3 | Model+UI: row count text | CardModel, MenuModel, Sources/AIUsage/CardView | todo |
| T4 | copilot.json revision 5 | Resources/providers/copilot.json | todo |
| T5 | Docs: resetsAt.from and count | example/AGENTS.md, README.md | todo |

### Briefs (each: file, one change, acceptance, constraints)
T1. File: Sources/CoreChecks (ModelChecks.swift, RunnerChecks.swift). Change: add checks, read specs first. Cover: (a) resetsAt.from root reads reset from the top-level answer; (b) count parse: limit 0 or missing, remaining missing, conditional unit matches only when condition true, overage unclamped (used = limit - remaining); (c) row pill text "63% · 6280/10000 credits" and "63% · 120/300" (no unit); (d) collapsed pills percent-only. Acceptance: `swift build` passes; `swift run CoreChecks` fails only on the new checks (compile errors on missing types are acceptable only if stubs are not added; prefer checks that compile and fail). Constraints: no edits outside Sources/CoreChecks; no specs/ or CHANGES.md edits.
T2. Files: ProviderConfig.swift, Report.swift, ProviderRunner.swift. Change: support resetsAt.from "root" and window count (limit, remaining, optional conditional unit); used = limit - remaining, not clamped. Acceptance: T1 parse/reset checks pass; build passes. Constraints: no UI files; no specs/CHANGES edits; do not change CoreChecks expectations.
T3. Files: CardModel.swift, MenuModel.swift, Sources/AIUsage/CardView.swift. Change: row pill text "63% · 6280/10000 credits" / "63% · 120/300"; pill sizing and drawing fit the longer text; collapsed pills stay percent-only. Acceptance: all CoreChecks pass; build passes. Constraints: no spec edits.
T4. File: Resources/providers/copilot.json. Change: revision 4 to 5; resetsAt {from root, path /quota_reset_date_utc, iso8601} on all 3 meters; premium count limit /entitlement, remaining /quota_remaining, unit "credits" only when /token_based_billing is true; premium match has_quota true and unlimited false. Acceptance: JSON valid; build and CoreChecks pass. Constraints: one file; no spec edits.
T5. Files: example/AGENTS.md, README.md. Change: document resetsAt.from and count in provider-format sections. Acceptance: text matches spec data-source.md wording. Constraints: docs only; no specs/CHANGES edits.

## Per-task result
(none yet)

## Final summary
(pending)

## Open issues
- No Coder/Tester/Reviewer connected yet (only RL). Asked human to recruit via Repo Lead.
- Spec edits: task brief says nobody edits specs/, so no repo-spec tasks.
