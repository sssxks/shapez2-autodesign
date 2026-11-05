param(
    [ValidateRange(5, 120)][int]$Seconds = 30,
    [string]$OutputRoot = "$env:TEMP/shapez-lab/performance"
)

$ErrorActionPreference = 'Stop'
$game = Get-Process -Name 'shapez 2' | Select-Object -First 1
$runDirectory = Join-Path $OutputRoot (Get-Date -Format 'yyyyMMdd-HHmmss')
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
$tracePath = Join-Path $runDirectory 'cpu-gpu.etl'

# WPR rejects a second recording; never cancel someone else's session.
& sudo wpr -start GeneralProfile -start GPU -filemode
if ($LASTEXITCODE -ne 0) { throw 'WPR could not start; inspect wpr -status.' }
try {
    [pscustomobject]@{
        ProcessId = $game.Id
        Executable = $game.Path
        StartedUtc = [DateTime]::UtcNow.ToString('O')
        DurationSeconds = $Seconds
        Profiles = @('GeneralProfile', 'GPU')
    } | ConvertTo-Json | Set-Content (Join-Path $runDirectory 'capture.json') -Encoding utf8
    Write-Host "Recording PID $($game.Id) for $Seconds seconds. Keep the game foreground."
    Start-Sleep -Seconds $Seconds
}
finally {
    & sudo wpr -stop $tracePath 'shapez 2 CPU and GPU capture'
    if ($LASTEXITCODE -ne 0) { throw 'WPR failed to save the trace; inspect wpr -status.' }
}
Get-Item -LiteralPath $tracePath | Select-Object FullName, Length
