# Windows provider runner

Windows executable lookup uses PATHEXT for bare command names, avoiding npm's extensionless POSIX scripts. The Codex Windows override sends `params: null` for `account/rateLimits/read`, as required by the installed CLI. Live x64 validation on 2026-10-10 returned a Codex usage meter successfully. Claude's CLI returned `rate_limits_available: false`; no usage percentage was supplied. Diagnostics represent missing percentages as `None`, rather than 0%.

Provider logic reads shared JSON definitions, with a Windows Copilot override. Each capture uses a fresh temporary directory, bounded output and line sizes, a whole-operation timeout, and an owned Job Object that terminates its subprocess tree. At most two captures run concurrently. One 30-second schedule timer handles intervals and retry backoff; refresh during a running capture queues one follow-up.

The environment is constructed from named required variables, with case-insensitive overrides and no inherited `GH_TOKEN` or `GITHUB_TOKEN`. Copilot's PowerShell child obtains its own CLI token and keeps it inside that child; the application never reads or stores it. Error messages do not expose captured output. Fixtures cover command and correlated stdio capture, output caps, timeouts including blocked stdin, and the Copilot adapter. Real account/provider sessions have not been tested on this host.
