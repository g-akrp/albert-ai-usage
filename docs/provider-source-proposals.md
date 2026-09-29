---
title: Provider Source Proposals
description: Unconfirmed research proposals for future provider usage retrieval.
---

# Provider source proposals

This document records research-grade source proposals only. Every source and field detail is **unconfirmed**; none is an approved implementation contract. No provider client/server command was run and no live capture was performed for this document.

| Provider    | Proposed source (unconfirmed)                                                  | Confidence                          | Status and limits                                                                        |
| ----------- | ------------------------------------------------------------------------------ | ----------------------------------- | ---------------------------------------------------------------------------------------- |
| Codex       | App-server JSON-RPC `account/rateLimits/read` after `initialize` — unconfirmed | High confidence, unconfirmed        | Candidate usage source; protocol and returned fields are unverified.                     |
| Copilot     | `gh api copilot_internal/user` premium quota snapshot — unconfirmed            | Medium confidence, unconfirmed      | Auth-method-dependent; availability and returned fields are unverified.                  |
| Antigravity | Local gRPC/Connect `GetUserStatus` — unconfirmed                               | Medium-high confidence, unconfirmed | Proposed local source; exact method behavior and field names are unverified.             |
| Claude Code | No supported source proposed; source assessment unconfirmed                    | Unsupported for V0.1.0              | No local non-interactive source exists. TUI scraping is forbidden and is not a fallback. |

## Verification gate

Live captures are a required precondition for proposing any exact provider-field allowlist. Until a separate live-capture verification task completes and the human approves the result, do not treat proposed source details or inferred fields as implementation-approved. Keep V0.1.0 source work limited to contracts that pass that gate.

## Data handling boundary

Do not access, log, or copy credentials, tokens, authentication state, raw responses, account identifiers, or unallowlisted response values into this document or repository. A future authorized verification process must preserve this boundary; this document contains no secrets or captured response values.
