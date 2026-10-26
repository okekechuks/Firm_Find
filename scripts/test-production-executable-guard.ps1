$ErrorActionPreference = 'Stop'
# Deliberately make the CI-missing cmdlet unusable. Only disposable bytes are tested.
function Get-FileHash { throw 'Get-FileHash must not be required' }
. (Join-Path $PSScriptRoot 'production-executable-guard.ps1')
$taskFixture = Join-Path ([System.IO.Path]::GetTempPath()) ('obtainer-exe-guard-' + [guid]::NewGuid().ToString('N'))
try {
    [System.IO.File]::WriteAllBytes($taskFixture, [byte[]](97,98,99))
    $taskExpected = 'BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD'
    if ((Get-ProductionExecutableHash $taskFixture) -ne $taskExpected) { throw 'SHA-256 vector differs' }
    Assert-ProductionExecutableUnchanged $taskFixture $taskExpected
    [System.IO.File]::WriteAllBytes($taskFixture, [byte[]](97,98,100))
    $taskRejected = $false
    try { Assert-ProductionExecutableUnchanged $taskFixture $taskExpected } catch { $taskRejected = $true }
    if (-not $taskRejected) { throw 'Changed production bytes were accepted' }
    [System.IO.File]::Delete($taskFixture)
    $taskRejected = $false
    try { Assert-ProductionExecutableUnchanged $taskFixture $taskExpected } catch { $taskRejected = $true }
    if (-not $taskRejected) { throw 'Missing production executable was accepted' }
    Write-Output 'SHA-256 vector, unchanged, changed and missing-file checks completed without Get-FileHash.'
} finally {
    # Exact generated file only; no directory traversal or recursive cleanup.
    [System.IO.File]::Delete($taskFixture)
}
