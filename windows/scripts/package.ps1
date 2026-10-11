param([ValidateSet('x64','arm64','all')][string]$Arch='all',[switch]$PortableOnly)
$ErrorActionPreference='Stop'
$windowsRoot=Split-Path -Parent $PSScriptRoot
$manifest=Get-Content -LiteralPath (Join-Path $windowsRoot 'Cargo.toml') -Raw
$version=[regex]::Match($manifest,'(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if(-not $version){throw 'Generated workspace version not found'}
$architectures=if($Arch -eq 'all'){@('x64','arm64')}else{@($Arch)}
$dist=Join-Path $windowsRoot 'dist'
New-Item -ItemType Directory -Path $dist -Force | Out-Null
Push-Location -LiteralPath $windowsRoot
try {
    foreach($architecture in $architectures){
        $exe=Join-Path $dist "$architecture\ai-usage.exe"
        if(-not(Test-Path -LiteralPath $exe)){throw "Build $architecture before packaging"}
        $zip=Join-Path $dist "AIUsage-win-$version-$architecture.zip"
        Compress-Archive -LiteralPath $exe -DestinationPath $zip -Force
        Write-Output "Portable ZIP: $zip"
        if(-not $PortableOnly){
            & dotnet tool run wix -- build installer/Package.wxs -arch $architecture -d "Version=$version" -d "Exe=$exe" -o "dist/AIUsage-win-$version-$architecture.msi"
            if($LASTEXITCODE -ne 0){throw 'WiX MSI build failed. Review WiX 7 EULA/OSMF acceptance; this script never accepts terms.'}
        }
    }
    $assets=Get-ChildItem -LiteralPath $dist -File | Where-Object Name -Match "^AIUsage-win-$([regex]::Escape($version))-(x64|arm64)\.(zip|msi)$" | Sort-Object Name
    $lines=$assets | ForEach-Object { $hash=Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256; "$($hash.Hash.ToLowerInvariant())  $($_.Name)" }
    [IO.File]::WriteAllLines((Join-Path $dist 'SHA256SUMS'),[string[]]$lines,[Text.UTF8Encoding]::new($false))
} finally {Pop-Location}
