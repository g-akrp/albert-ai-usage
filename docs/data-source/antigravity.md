---
title: Antigravity data source
description: Investigated, real mechanism found, live query blocked -- documented honestly rather than guessed.
date: 2026-09-29
---

# Antigravity (agy) data source

Investigated 2026-09-29 against `agy` (Antigravity CLI) `1.2.12`, locally installed. **Unlike Codex and Copilot, this one is not working yet** — real blockers found, documented here instead of shipping a guess.

## What's confirmed

The CLI hosts a local server per invocation, logged to `~/.gemini/antigravity-cli/cli.log` (a symlink, **repointed to a fresh file each run** — not appended; any tooling that reads it must re-resolve the symlink per invocation, not track a byte offset):

```
Language server will attempt to listen on host localhost
Language server listening on random port at <PORT> for HTTPS (gRPC)
Language server listening on random port at <PORT> for HTTP
```

Ports are random per invocation — no fixed port to hardcode. The earlier research proposal's endpoint path, `/exa.language_server_pb.LanguageServerService/GetUserStatus`, is real (the package/service name appears nowhere disprovable), but **no query against it succeeded**.

Real upstream calls observed in the log (so quota data does flow through the process, just not confirmed reachable locally): `https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist`, `.../streamGenerateContent`, and an internal `quota_manager.go: doRefreshQuota` call — Google's Cloud Code Assist backend, same family Gemini CLI uses.

## What's blocked, and the likely reason

Every `curl -X POST http://localhost:<port>/.../GetUserStatus` attempt got connection-refused (`CODE=000`) immediately, even when the log showed the port as bound and auth as succeeded. Most likely cause: **the "HTTP" port is not necessarily plain HTTP/1.1** — Go gRPC servers are typically HTTP/2-only, and Connect-protocol JSON (which does work over HTTP/1.1) may not be what this particular port serves. `curl` can't negotiate real gRPC; the right tool is `grpcurl`, which isn't installed on this machine and wasn't installed as part of this investigation (that's a real environment change, not a read).

Secondary complication: the server's lifetime is tied to the CLI invocation — a `--print`/`-i` call exits in a few seconds, leaving a narrow window to query it, and scripted attempts to keep it alive (piped stdin tricks) proved unreliable in this harness. A one-shot `agy models` call proved auth generally works (returned real model list data quickly), but that's a different code path than the quota/status endpoint, which intermittently logged `error getting token source: You are not logged into Antigravity` even in sessions where other calls succeeded — auth for this specific endpoint looks flakier than for Codex or Copilot.

## Recommendation for V0.1.0

Mark Antigravity `Unsupported`/`Pending`, same treatment as Claude Code — a real gap, not just unresearched. Unlike Claude Code, this one has a plausible path forward:

1. Install `grpcurl` (or add a Go/Rust gRPC client) and retry against the confirmed port/service/method with real HTTP/2, not `curl`.
2. Or capture the exact request agy's own UI/extension makes to this endpoint (packet capture or a debug flag), rather than guessing the Connect-JSON envelope.

Both are real next steps, not more guessing — flagging for whenever this is prioritized rather than shipping an adapter built on an unverified assumption.
