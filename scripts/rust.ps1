$CargoArguments = $args
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$localCargo = Join-Path $projectRoot '.tools\cargo'
if (Test-Path -LiteralPath (Join-Path $localCargo 'bin\cargo.exe')) {
    $env:CARGO_HOME = $localCargo
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    $env:PATH = "$localCargo\bin;$env:PATH"
}
Push-Location -LiteralPath $projectRoot
try {
    & cargo @CargoArguments
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
