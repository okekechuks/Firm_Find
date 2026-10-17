$ErrorActionPreference='Stop'
$taskRoot=Split-Path -Parent $PSScriptRoot
$taskNormal=Join-Path $taskRoot 'target\release\obtainer.exe'
$taskTest=Join-Path $taskRoot 'target\release\obtainer-native-test.exe'
if(-not (Test-Path -LiteralPath $taskNormal)){throw 'Build the normal production application first: npm run tauri -- build --no-bundle --ci -- --locked --offline'}
$taskOriginalHash=(Get-FileHash -LiteralPath $taskNormal -Algorithm SHA256).Hash
& npm.cmd run build
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
$taskPreviousConfig=$env:TAURI_CONFIG
try {
    $env:TAURI_CONFIG=Get-Content -LiteralPath (Join-Path $taskRoot 'apps\desktop\src-tauri\tauri.native-test.conf.json') -Raw
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'rust.ps1') build --release --features native-test-profile,tauri/custom-protocol --bin obtainer-native-test --locked --offline
    $taskExitCode=$LASTEXITCODE
} finally {$env:TAURI_CONFIG=$taskPreviousConfig}
if(-not (Test-Path -LiteralPath $taskNormal) -or (Get-FileHash -LiteralPath $taskNormal -Algorithm SHA256).Hash -ne $taskOriginalHash){throw 'Native-test build altered the normal executable. Stop and inspect build configuration.'}
if($taskExitCode -ne 0){exit $taskExitCode}
if(-not (Test-Path -LiteralPath $taskTest)){throw 'Selected native-test Cargo binary was not produced.'}
Write-Output "Dedicated native-test executable: $taskTest"
Write-Output "Normal executable preserved: $taskOriginalHash"
exit 0
