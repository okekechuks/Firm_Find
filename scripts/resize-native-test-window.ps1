param([Parameter(Mandatory=$true)][int]$OwnerPid,[Parameter(Mandatory=$true)][int]$Width,[Parameter(Mandatory=$true)][int]$Height)
$ErrorActionPreference='Stop'
$taskProcess=Get-Process -Id $OwnerPid
if($taskProcess.ProcessName -ne 'obtainer-native-test' -or $taskProcess.MainWindowHandle -eq 0){throw 'Only a visible dedicated native-test window may be resized.'}
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ObtainerWindowSize {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out Rect rect);
 [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hwnd,int x,int y,int width,int height,bool repaint);
}
'@
$taskRect=[ObtainerWindowSize+Rect]::new()
if(-not [ObtainerWindowSize]::GetWindowRect($taskProcess.MainWindowHandle,[ref]$taskRect)){throw 'Owned window bounds unavailable.'}
if(-not [ObtainerWindowSize]::MoveWindow($taskProcess.MainWindowHandle,$taskRect.Left,$taskRect.Top,$Width,$Height,$true)){throw 'Owned window resize failed.'}
[pscustomobject]@{Width=$taskRect.Right-$taskRect.Left;Height=$taskRect.Bottom-$taskRect.Top} | ConvertTo-Json -Compress
