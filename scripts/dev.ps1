param([Parameter(ValueFromRemainingArguments=$true)][string[]]$CargoArgs)
$ErrorActionPreference='Stop'
$env:CARGO_TARGET_DIR=Join-Path ([IO.Path]::GetTempPath()) 'shapez-lab\target'
Push-Location (Split-Path $PSScriptRoot)
try { & cargo @CargoArgs; exit $LASTEXITCODE } finally { Pop-Location }

