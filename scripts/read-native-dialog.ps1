param([Parameter(Mandatory=$true)][int]$OwnerPid,[Parameter(Mandatory=$true)][string]$Title)
$ErrorActionPreference='Stop'
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class ObservedDialog {
 public delegate bool Callback(IntPtr w, IntPtr l);
 [DllImport("user32.dll")] public static extern bool EnumWindows(Callback c, IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr w, Callback c, IntPtr p);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr w,StringBuilder b,int n);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr w,out uint p);
}
"@
$script:taskDialog=[IntPtr]::Zero
$taskDeadline=[DateTime]::UtcNow.AddSeconds(10)
while($script:taskDialog -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $taskDeadline){
 [ObservedDialog]::EnumWindows({param($w,$unused)
  $taskProcess=[uint32]0;[void][ObservedDialog]::GetWindowThreadProcessId($w,[ref]$taskProcess)
  $taskText=New-Object Text.StringBuilder 2048;[void][ObservedDialog]::GetWindowText($w,$taskText,2048)
  if($taskProcess -eq $OwnerPid -and $taskText.ToString() -eq $Title){$script:taskDialog=$w}
  return $true
 },[IntPtr]::Zero)|Out-Null
 if($script:taskDialog -eq [IntPtr]::Zero){Start-Sleep -Milliseconds 50}
}
if($script:taskDialog -eq [IntPtr]::Zero){throw 'Owned native dialog not found; no dialog action taken.'}
$script:taskChildren=New-Object 'System.Collections.Generic.List[string]'
[ObservedDialog]::EnumChildWindows($script:taskDialog,{param($w,$unused)
 $taskText=New-Object Text.StringBuilder 4096;[void][ObservedDialog]::GetWindowText($w,$taskText,4096)
 if($taskText.Length -gt 0){$script:taskChildren.Add($taskText.ToString())}
 return $true
},[IntPtr]::Zero)|Out-Null
[pscustomobject]@{title=$Title;ownerPid=$OwnerPid;observedAt=[DateTime]::UtcNow.ToString('o');children=$script:taskChildren.ToArray()}|ConvertTo-Json -Depth 3
