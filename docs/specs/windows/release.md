# Windows portable preview

Build with `windows/scripts/build.ps1 -Arch all`; package with `windows/scripts/package.ps1 -Arch all -PortableOnly`. Artifacts are `windows/dist/AIUsage-win-<version>-x64.zip`, the matching ARM64 ZIP, and `SHA256SUMS`. Each ZIP contains one statically linked CRT executable. Extract to a stable writable location and run it. Launch-at-login stores that executable path in the current user's Run key. Quit before replacing it for a manual update.

`windows/scripts/release.ps1` controls Windows versions and prepares local artifacts only. It does not commit, push, publish, install, or accept terms. Builds are unsigned. Users should keep provider CLI login under those tools' control.

Native core checks and x64/ARM64 compilation are automated. The x64 debug panel and refresh were manually observed; its private working set was approximately 11.6 MiB. Release 15-minute memory/handle endurance, full tray/DPI/keyboard behavior, real provider sessions, portable GUI smoke, and ARM64 runtime remain pending. No macOS checks were run on this Windows host.

The user selected portable ZIPs only. WiX 7 terms remain unaccepted; MSI source is retained as a draft. MSI build, install, upgrade, and uninstall are deferred and have not been verified.
