param([switch]$RequireMsi)
$ErrorActionPreference='Stop'
$windowsRoot=Split-Path -Parent $PSScriptRoot
$nativeReady=$true
foreach($tool in @('cargo','rustc','rustup')){
    $command=Get-Command $tool -ErrorAction SilentlyContinue
    if(-not $command){Write-Output "$tool missing";$nativeReady=$false}else{Write-Output "$tool found: $($command.Source)"}
}
if($nativeReady){
    $targets=& rustup target list --installed
    foreach($target in @('x86_64-pc-windows-msvc','aarch64-pc-windows-msvc')){if($targets -notcontains $target){Write-Output "Missing Rust target: $target";$nativeReady=$false}}
    $vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if(Test-Path -LiteralPath $vswhere){
        $installations=& $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if(-not $installations){$nativeReady=$false;Write-Output 'MSVC x64 tools missing'}
        $armInstallations=& $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath
        if(-not $armInstallations){$nativeReady=$false;Write-Output 'MSVC ARM64 tools missing'}
    }else{$nativeReady=$false;Write-Output 'Visual Studio build tools missing'}
    $sdk=Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Include'
    if(-not(Test-Path -LiteralPath $sdk)){$nativeReady=$false;Write-Output 'Windows SDK missing'}
}
$msiReady=$false
if(Get-Command dotnet -ErrorAction SilentlyContinue){
    Push-Location -LiteralPath $windowsRoot
    try {
        if(Test-Path -LiteralPath '.config\dotnet-tools.json'){
            $help=& dotnet tool run wix -- build --help 2>&1
            $msiReady=$LASTEXITCODE -eq 0
            if(-not $msiReady){Write-Output ($help | Out-String)}
        }else{Write-Output 'MSI tool manifest not restored yet'}
    } finally {Pop-Location}
}else{Write-Output '.NET SDK missing for MSI packaging'}
Write-Output "Native build ready: $nativeReady"
Write-Output "MSI packaging ready: $msiReady"
if(-not $nativeReady -or ($RequireMsi -and -not $msiReady)){exit 1}
