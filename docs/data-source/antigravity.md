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

## What's blocked, and exactly how far it got

**Update:** installed `grpcurl` (`brew install grpcurl`) and retried properly. This got materially further:

1. `curl` against the "HTTP" port: connection-refused every time. Confirmed reason — it's a real gRPC server, HTTP/2 + TLS only; `curl` can't negotiate that.
2. `grpcurl -plaintext`: `context deadline exceeded` — confirms TLS is required (matches the log's own "HTTPS (gRPC)" label).
3. `grpcurl -insecure` (TLS, self-signed cert accepted): **the connection succeeds.** The gRPC server is real and reachable.
4. `grpcurl -insecure list` (server reflection, to discover exact service/method names): `server does not support the reflection API`.
5. `grpcurl -insecure -d '{}' ... exa.language_server_pb.LanguageServerService/GetUserStatus`: `failed to query for service descriptor ... server does not support the reflection API` — grpcurl needs a `.proto` or compiled descriptor set to encode the request and decode the response when reflection is off, and it has neither.
6. Searched the agy installation for a bundled `.proto`/descriptor file: none found. The binary is a single compiled Mach-O executable; no proto sources ship alongside it.

**So the real remaining blocker is a missing schema, not a missing service.** The service exists, is reachable over TLS, and (per the earlier research) the method name is very likely correct — but nothing on this machine can encode/decode its messages without either the `.proto` source or a hand-reconstructed one with correct field numbers, which would require reverse-engineering the wire format from raw bytes (a materially larger, more specialized effort than confirming Codex or Copilot).

Secondary complication, still real: the server's lifetime is tied to the CLI invocation (a few seconds for `--print`/`-i`/`models`), so any future attempt needs to grab the port and query fast; auth for this quota/status path was also observed to intermittently fail (`error getting token source: You are not logged into Antigravity`) even in sessions where other calls (`agy models`) succeeded.

## Recommendation for V0.1.0

Mark Antigravity `Unsupported`/`Pending`, same treatment as Claude Code — a real, now precisely-bounded gap. Next step, if pursued: obtain or reconstruct the `.proto` schema for `exa.language_server_pb.LanguageServerService` (upstream source, a packet capture of a real client's request/response, or careful manual protobuf wire-format decoding) — not more guessing at the transport, which is now fully confirmed.
