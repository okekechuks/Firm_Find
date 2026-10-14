$DesktopArguments = $args
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$localCargo = Join-Path $projectRoot '.tools\cargo'
$localRustup = Join-Path $projectRoot '.tools\rustup'
if (Test-Path -LiteralPath (Join-Path $localCargo 'bin\cargo.exe')) {
    $env:CARGO_HOME = $localCargo
    $env:RUSTUP_HOME = $localRustup
    $env:PATH = "$localCargo\bin;$env:PATH"
}
Push-Location -LiteralPath $projectRoot
try {
    & node (Join-Path $projectRoot 'node_modules\@tauri-apps\cli\tauri.js') @DesktopArguments
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
