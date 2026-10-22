# Local native GUI qualification only. No persistent power-plan changes.
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Windows.Forms
if ([System.Windows.Forms.SystemInformation]::PowerStatus.PowerLineStatus -ne [System.Windows.Forms.PowerLineStatus]::Online) {
    throw 'Connect AC power before running timed native qualification on this machine.'
}
Add-Type @'
using System.Runtime.InteropServices;
public static class NativeVerificationPower {
    [DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint state);
}
'@
$held=[NativeVerificationPower]::SetThreadExecutionState([uint32]2147483651) # CONTINUOUS | SYSTEM_REQUIRED | DISPLAY_REQUIRED
if ($held -eq 0) { throw 'Could not acquire temporary execution state for native GUI verification.' }
$result=1
Push-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
try {
    Write-Output 'Native verification: temporary idle-sleep hold active; power plan unchanged.'
    & npm.cmd run test:openwrt
    $result=$LASTEXITCODE
} finally {
    [void][NativeVerificationPower]::SetThreadExecutionState([uint32]2147483648)
    Write-Output 'Native verification: temporary idle-sleep hold cleared.'
    Pop-Location
}
exit $result
