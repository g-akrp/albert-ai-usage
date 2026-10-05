---
name: backlog-idea
description: Use when the user wants to add a new idea, feature request, or change proposal to this repo's backlog, or asks to advance a proposal from docs/inbox through review, spec, implementation, or spec sync.
---

# Backlog Idea Lifecycle

## Overview

Every idea moves through four stages, each with a fixed home in `docs/`. The folder a file lives in is its status. Never skip a stage and never implement from `docs/inbox/`.

| Stage | Location | Exit condition |
|-------|----------|----------------|
| 1. Proposal | `docs/inbox/<slug>.md` | User reviewed and approved the revised proposal |
| 2. Approved change | `docs/changes/<slug>.md` (moved with `git mv`) | Implementation spec / plan written in the same file |
| 3. Implement | code, following the plan | Plan tasks done and verified |
| 4. Spec sync | `docs/spec/` updated | Spec describes the shipped behavior |

`<slug>` is short kebab-case, e.g. `windows-tray-icon`. Create `docs/inbox/`, `docs/changes/`, `docs/spec/` if missing.

## Stage 1: Write the proposal

Write `docs/inbox/<slug>.md`. Stop after writing and ask the user to review. Do not write code or a plan yet.

```markdown
# <Title>

Status: proposed
Date: <YYYY-MM-DD>

## Problem
What hurts today, for whom.

## Proposal
What to build, at behavior level.

## Scope
In / out.

## Alternatives considered

## Impact
Rust core, Swift/AppKit shell, C#/WinUI shell, `docs/core-abi.md`, `docs/architecture.md`.

## Open questions
```

Check the proposal against `AGENTS.md` before presenting it. If it needs Electron/Tauri/Wails or another cross-platform GUI, business logic outside the Rust core, or OS credential storage / tray / launch-at-login decisions, list that under Open questions as needing separate approval. Do not decide it in the proposal.

## Stage 2: Review and revise, then promote

Revise the proposal in place from the user's feedback until they approve. Only then:

1. `git mv docs/inbox/<slug>.md docs/changes/<slug>.md`
2. Set `Status: approved`.
3. Append the implementation spec / plan (below).

Without explicit approval, the file stays in `docs/inbox/`.

## Stage 3: Implementation spec / plan, then implement

Append to the change file:

```markdown
## Implementation spec
Concrete design: types, ABI changes, data flow, files touched.

## Plan
- [ ] Task 1 (file paths, verification command)
- [ ] ...

## Verification
Commands that prove it works.
```

Any Rust/shell ABI change goes into `docs/core-abi.md` first and bindings are generated per its rules, never hand-copied. Keep logic in the Rust core. For multi-step plans use `superpowers:writing-plans` and `superpowers:executing-plans`; use `superpowers:test-driven-development` and `superpowers:verification-before-completion` while implementing. Tick plan boxes as tasks finish and set `Status: implemented` at the end.

## Stage 4: Update `docs/spec`

After implementation is verified, update `docs/spec/` so it describes current behavior, not the proposal history. Edit the existing topic file if one fits; add a new one otherwise. Spec states what the system does now: no "will", no discussion of alternatives. Link back to `docs/changes/<slug>.md` for rationale. Check `docs/architecture.md` and `docs/core-abi.md` for statements the change made stale and fix them.

## Common mistakes

- Writing the plan or code while the file is still in `docs/inbox/`.
- Moving to `docs/changes/` before the user approved.
- Marking done without updating `docs/spec/`.
- Spec that reads like a proposal instead of current behavior.
