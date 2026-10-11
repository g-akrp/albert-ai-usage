param([ValidateSet('x64','arm64','all')][string]$Arch='all')
$ErrorActionPreference='Stop'
$windowsRoot=Split-Path -Parent $PSScriptRoot
$architectures=if($Arch -eq 'all'){@('x64','arm64')}else{@($Arch)}
Push-Location -LiteralPath $windowsRoot
try {
    foreach($architecture in $architectures){
        $target=if($architecture -eq 'x64'){'x86_64-pc-windows-msvc'}else{'aarch64-pc-windows-msvc'}
        & cargo build --workspace --release --locked --target $target
        if($LASTEXITCODE -ne 0){throw "Cargo build failed for $architecture"}
        $destination=Join-Path $windowsRoot "dist\$architecture"
        New-Item -ItemType Directory -Path $destination -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $windowsRoot "target\$target\release\ai-usage.exe") -Destination (Join-Path $destination 'ai-usage.exe')
        Write-Output "$architecture executable: $(Join-Path $destination 'ai-usage.exe')"
    }
} finally { Pop-Location }
