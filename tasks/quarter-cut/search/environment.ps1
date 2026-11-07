# Dot-source before compiling/running the optional native search executable.
$ErrorActionPreference = 'Stop'
$solverVersion = '9.15.6755'
$solverFolder = "or-tools_x64_VisualStudio2022_cpp_v$solverVersion"
$solverRoot = Join-Path $env:TEMP 'shapez-lab/tools/ortools-9.15'
if (-not $env:ORTOOLS_PREFIX) {
    $solverPrefix = Join-Path $solverRoot $solverFolder
    if (-not (Test-Path -LiteralPath (Join-Path $solverPrefix 'lib/ortools.lib'))) {
        New-Item -ItemType Directory -Force $solverRoot | Out-Null
        $solverArchive = Join-Path $solverRoot 'ortools.zip'
        Invoke-WebRequest "https://github.com/google/or-tools/releases/download/v9.15/$solverFolder.zip" -OutFile $solverArchive
        Expand-Archive -LiteralPath $solverArchive -DestinationPath $solverRoot -Force
    }
    $env:ORTOOLS_PREFIX = $solverPrefix
}
$solverBin = Join-Path $env:ORTOOLS_PREFIX 'bin'
if (-not $env:PROTOC) { $env:PROTOC = Join-Path $solverBin 'protoc.exe' }
$env:PATH = "$solverBin;$env:PATH"
