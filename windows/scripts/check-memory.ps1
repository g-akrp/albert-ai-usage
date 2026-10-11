param([Parameter(Mandatory=$true)][int]$ProcessId,[ValidateRange(1,900)][int]$Samples=1,[ValidateRange(1,60)][int]$IntervalSeconds=5)
$ErrorActionPreference='Stop'
for($sample=0;$sample -lt $Samples;$sample++){
    $metrics=Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object IDProcess -eq $ProcessId
    if(-not $metrics){throw "Owned process $ProcessId not found"}
    $privateMiB=[math]::Round($metrics.WorkingSetPrivate/1MB,2)
    [pscustomobject]@{ProcessId=$ProcessId;PrivateWorkingSetMiB=$privateMiB;Handles=$metrics.HandleCount;Passed=$metrics.WorkingSetPrivate -lt 30MB}
    if($metrics.WorkingSetPrivate -ge 30MB){throw 'Application private working set exceeds 30 MiB'}
    if($sample+1 -lt $Samples){Start-Sleep -Seconds $IntervalSeconds}
}
