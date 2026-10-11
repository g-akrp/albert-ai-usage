# Windows agent instructions

- Follow `docs/changes/windows-tray-app.md`. Implement fresh Rust/Win32; never restore the historical Rust core.
- Runtime dependencies are `windows`, `serde_json`, and the local core crate. No GUI framework, web view, helper daemon, async runtime, or third-party date library.
- Write core checks first. Verify with Cargo tests, rustfmt, clippy, and both MSVC targets. Keep `Cargo.lock` for reproducible builds.
- Preserve current macOS behavior and shared provider JSON. Platform adapters belong in `Resources/providers-windows/`.
- Never read, store, or log credentials. Strip inherited GH_TOKEN/GITHUB_TOKEN. Copilot's CLI-owned child handles its own authentication.
- One 30-second schedule timer; at most two concurrent captures. Bound output and kill owned process trees on timeout/quit.
- Release app private working set must stay below 30 MiB; measure actual release processes.
- WiX 7 is build-time only. Never accept its EULA automatically. Native/portable work proceeds if MSI packaging is blocked.
- Versions are generated through `scripts/release.ps1`. No unsolicited commit/tag/push/publish or installation.
