param([Parameter(Mandatory=$true)][string]$Version,[switch]$PortableOnly,[switch]$DryRun)
$ErrorActionPreference='Stop'
if($Version -notmatch '^([0-9]+)\.([0-9]+)\.([0-9]+)$'){throw 'Version must be major.minor.patch'}
if([int64]$Matches[1] -gt 255 -or [int64]$Matches[2] -gt 255 -or [int64]$Matches[3] -gt 65535){throw 'Version exceeds MSI component limits'}
$windowsRoot=Split-Path -Parent $PSScriptRoot
$repoRoot=Split-Path -Parent $windowsRoot
$manifestPath=Join-Path $windowsRoot 'Cargo.toml'
$manifest=Get-Content -LiteralPath $manifestPath -Raw
$updated=[regex]::Replace($manifest,'(?m)^version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+"',"version = `"$Version`"")
if($DryRun){Write-Output "Would set Windows version to $Version, verify, build and package locally. No Git, install or EULA actions.";return}
Push-Location -LiteralPath $repoRoot
try {
    $status=& git status --porcelain
    if($status){throw 'Release preparation requires a clean checkout. Commit only when explicitly requested.'}
} finally {Pop-Location}
$lockPath=Join-Path $windowsRoot 'Cargo.lock'
$lock=if(Test-Path -LiteralPath $lockPath){[IO.File]::ReadAllBytes($lockPath)}else{$null}
Push-Location -LiteralPath $windowsRoot
try {
    [IO.File]::WriteAllText($manifestPath,$updated,[Text.UTF8Encoding]::new($false))
    & cargo check --workspace
    if($LASTEXITCODE -ne 0){throw 'Cargo version update check failed'}
    & cargo fmt --all -- --check
    if($LASTEXITCODE -ne 0){throw 'Formatting failed'}
    & cargo clippy --workspace --all-targets -- -D warnings
    if($LASTEXITCODE -ne 0){throw 'Static analysis failed'}
    & cargo test --workspace
    if($LASTEXITCODE -ne 0){throw 'Checks failed'}
    & (Join-Path $PSScriptRoot 'build.ps1') -Arch all
    & (Join-Path $PSScriptRoot 'package.ps1') -Arch all -PortableOnly:$PortableOnly
} catch {
    [IO.File]::WriteAllText($manifestPath,$manifest,[Text.UTF8Encoding]::new($false))
    if($null -ne $lock){[IO.File]::WriteAllBytes($lockPath,$lock)}
    throw
} finally {Pop-Location}
