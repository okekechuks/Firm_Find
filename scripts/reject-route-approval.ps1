param([Parameter(Mandatory=$true)][int]$OwnerPid)
$ErrorActionPreference = 'Stop'
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class RouteApprovalWindow {
 public delegate bool Callback(IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern bool EnumWindows(Callback cb,IntPtr p);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr w,StringBuilder b,int n);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr w,out uint p);
 [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr w,int id);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr w,uint m,IntPtr a,IntPtr b);
}
"@
$taskDeadline=[DateTime]::UtcNow.AddSeconds(15)
$script:taskApproval=[IntPtr]::Zero
while ($script:taskApproval -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $taskDeadline) {
 [RouteApprovalWindow]::EnumWindows({param($taskWindow,$unused)
  $taskProcess=[uint32]0;[void][RouteApprovalWindow]::GetWindowThreadProcessId($taskWindow,[ref]$taskProcess)
  $taskTitle=New-Object Text.StringBuilder 512;[void][RouteApprovalWindow]::GetWindowText($taskWindow,$taskTitle,512)
  if ($taskProcess -eq $OwnerPid -and $taskTitle.ToString() -eq 'OBTAINER - approve read-only check') {$script:taskApproval=$taskWindow}
  return $true
 },[IntPtr]::Zero) | Out-Null
 if ($script:taskApproval -eq [IntPtr]::Zero) {Start-Sleep -Milliseconds 100}
}
if ($script:taskApproval -eq [IntPtr]::Zero) {throw 'Owned native route-approval dialog did not appear.'}
$taskNo=[RouteApprovalWindow]::GetDlgItem($script:taskApproval,7)
if ($taskNo -eq [IntPtr]::Zero) {throw 'Native No button missing.'}
if (-not [RouteApprovalWindow]::PostMessage($taskNo,0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)) {throw 'Could not click native No button.'}
Write-Output 'Rejected native endpoint approval; no network operation authorized.'
