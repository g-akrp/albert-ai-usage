---
name: install-ai-usage
description: Build and install or update this repository's AI Usage Windows app on the current device. Use for local install, reinstall, or update requests; not for publishing releases or installing provider CLIs.
---

# Install AI Usage on this device

Install the current checkout's Windows executable for the current user and verify startup. A request to install or update authorizes installation; creating or editing this skill does not.

## Prepare and build

- Locate the repository from this skill's directory (`../../..`). Read root and `windows/AGENTS.md`, `README.md`, and `windows/scripts/build.ps1`.
- Require Windows. Detect OS architecture using `[System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture`: X64 -> x64, Arm64 -> arm64. Stop for unsupported architectures.
- Build current source, including uncommitted changes. Never reset source, change versions, commit, push, publish, or invoke `release.ps1`.
- Use portable per-user installation. MSI is deferred; do not install WiX or accept its terms.
- Find existing installation through the app-specific `InstallPath` value under `HKCU:\Software\g.akrp\AI Usage`, the AI Usage launch-at-login entry, and paths of running `ai-usage` processes. Inspect only app-specific values. If an MSI-managed installation exists, explain the conflict and ask for the update route before overwriting managed files.
- Reuse a clearly identified portable install. Default new destination: `$env:LOCALAPPDATA\Programs\AI Usage\ai-usage.exe`. Ask which to update if multiple installs make the target ambiguous.
- Preserve `%APPDATA%\ai-usage`, provider overrides, and launch-at-login preference. Never inspect credentials or CLI authentication files.
- From repository root run `windows/scripts/build.ps1 -Arch <x64|arm64>`. Require success; use `windows/dist/<arch>/ai-usage.exe`. Check its `--version` command. Sandbox access denial -> normal escalation flow, not code changes.

## Install

1. Complete build before quitting the installed app. Resolve absolute source/destination paths; verify destination stays in the chosen install directory and differs from build output.
2. Gracefully quit any process running that exact target executable via **Quit AI Usage** in its tray menu, or post `WM_CLOSE` to its `AIUsageWindowsNative` window after checking window PID and executable path. Wait with a bounded timeout. `CloseMainWindow()` can miss the hidden tray window. Do not kill all processes named `ai-usage`; if quit fails, stop before replacement.
3. Create install directory. Back up previous executable to a unique filename there, then copy the built executable using PowerShell literal paths. Compare SHA-256 hashes. Restore backup on failure; never delete settings.
4. Create or update a per-user Start Menu **AI Usage** shortcut targeting the installed executable, using install directory as working directory and executable as icon source. Do not add desktop shortcuts or enable launch-at-login unless requested. Preserve existing startup preference; if an enabled app entry must move, update only its quoted executable path.
5. Launch installed executable with `Start-Process -FilePath <absolute-exe> -WorkingDirectory <install-directory> -WindowStyle Hidden -PassThru`. Do not use demo mode for installation.

Use normal sandbox escalation for writes outside the workspace. Do not bypass rejection using another tool.

## Verify

- Check installed `--version`, matching source/destination hashes, and surviving process with exact installed executable path. An older singleton process is not evidence the updated app started.
- After one normal 30-second refresh interval, run `windows/scripts/check-memory.ps1 -ProcessId <installed-pid>`; private working set must remain below 30 MiB. Report unavailable metrics or startup failures. Claim tray UI verification only if observed.
- If startup fails, restore previous executable and restart it if previously running. Keep backup until verification succeeds; afterward remove only backup created by this run.
- Report installed path, version, architecture, and verification result. Missing provider CLI login is separate from installation success; login remains CLI-owned.
