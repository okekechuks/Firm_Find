param([Parameter(Mandatory=$true)][string]$Binary,[string]$ExpectedText='already running',[switch]$ActivationProbe,[switch]$OpenWrtLab)
$ErrorActionPreference='Stop'
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class StartupWindow {
  public delegate bool Callback(IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(Callback cb,IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr w,Callback cb,IntPtr p);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr w,StringBuilder b,int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr w,out uint p);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr w);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr w,uint m,IntPtr a,IntPtr b);
}
'@
$localDirectory=[Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
$hashAlgorithm=[Security.Cryptography.SHA256]::Create()
$hash=([BitConverter]::ToString($hashAlgorithm.ComputeHash([Text.Encoding]::UTF8.GetBytes($localDirectory.ToLowerInvariant()+'|production')))).Replace('-','').ToLowerInvariant()
$hashAlgorithm.Dispose()
$created=$false
$mutex=New-Object Threading.Mutex($false,('Local\OBTAINER-'+$hash),[ref]$created)
if (-not $created) { $mutex.Dispose(); throw 'Existing production owner; startup probe refused to interfere.' }
$child=$null
$savedRoot=$env:OBTAINER_TEST_ROOT; $savedId=$env:OBTAINER_NATIVE_TEST_RUN_ID
try {
  # Force the ordinary binary into the existing-owner failure before user data opens.
  if (-not $ActivationProbe) {
    Remove-Item Env:OBTAINER_TEST_ROOT -ErrorAction SilentlyContinue
    Remove-Item Env:OBTAINER_NATIVE_TEST_RUN_ID -ErrorAction SilentlyContinue
  }
  if ($OpenWrtLab) { $child=Start-Process -FilePath $Binary -ArgumentList '--openwrt-lab' -WindowStyle Hidden -PassThru }
  else { $child=Start-Process -FilePath $Binary -WindowStyle Hidden -PassThru }
  $script:startupDialog=[IntPtr]::Zero
  $deadline=[DateTime]::UtcNow.AddSeconds(15)
  while ($script:startupDialog -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline) {
    [StartupWindow]::EnumWindows({param($window,$unused)
      $ownerId=[uint32]0;[void][StartupWindow]::GetWindowThreadProcessId($window,[ref]$ownerId)
      $title=New-Object Text.StringBuilder 512;[void][StartupWindow]::GetWindowText($window,$title,512)
      if ($ownerId -eq $child.Id -and [StartupWindow]::IsWindowVisible($window) -and $title.ToString().StartsWith('OBTAINER') -and $title.ToString().Contains('startup failed')) { $script:startupDialog=$window }
      return $true
    },[IntPtr]::Zero) | Out-Null
    if ($script:startupDialog -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 50 }
  }
  if ($script:startupDialog -eq [IntPtr]::Zero) { throw 'Actual release startup dialog was not found.' }
  $script:startupText=''
  [StartupWindow]::EnumChildWindows($script:startupDialog,{param($window,$unused)
    $text=New-Object Text.StringBuilder 4096;[void][StartupWindow]::GetWindowText($window,$text,4096);$script:startupText+=$text.ToString()+"`n";return $true
  },[IntPtr]::Zero) | Out-Null
  if (-not $script:startupText.Contains($ExpectedText) -or -not $script:startupText.Contains('Existing data has not been reset')) { throw ('Startup dialog did not explain expected failure: '+$ExpectedText) }
  if (-not [StartupWindow]::PostMessage($script:startupDialog,0x0010,[IntPtr]::Zero,[IntPtr]::Zero)) { throw 'Could not close the owned startup dialog.' }
  if (-not $child.WaitForExit(10000)) { throw 'Startup error dialog did not release the process.' }
  if ($child.ExitCode -ne 1) { throw 'Expected startup failure exit code 1.' }
  [pscustomobject]@{actualProductionWindow=$true;cause=$ExpectedText;exitCode=$child.ExitCode;message=$script:startupText;normalDataOpened=$false} | ConvertTo-Json
} finally {
  $env:OBTAINER_TEST_ROOT=$savedRoot; $env:OBTAINER_NATIVE_TEST_RUN_ID=$savedId
  if ($child -and -not $child.HasExited) { [void]$child.CloseMainWindow(); if (-not $child.WaitForExit(3000)) { $child.Kill(); $child.WaitForExit() } }
  $mutex.Dispose()
}
