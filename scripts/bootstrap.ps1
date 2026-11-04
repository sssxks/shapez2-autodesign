$ErrorActionPreference='Stop'
$toolRoot=Join-Path ([IO.Path]::GetTempPath()) 'shapez-lab\tools'
New-Item -ItemType Directory -Force $toolRoot | Out-Null
$installer=Join-Path $toolRoot 'dotnet-install.ps1'
Invoke-WebRequest 'https://dot.net/v1/dotnet-install.ps1' -OutFile $installer
& $installer -Version '10.0.403' -InstallDir (Join-Path $toolRoot 'dotnet') -NoPath
if($LASTEXITCODE -ne 0){throw 'SDK installation failed'}
