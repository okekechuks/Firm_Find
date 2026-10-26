param([Parameter(Mandatory=$true)][int]$LauncherProcessId,
      [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{32}$')][string]$RunId,
      [Parameter(Mandatory=$true)][ValidateSet('Yes','No')][string]$Answer)
$ErrorActionPreference='Stop'
if($env:OBTAINER_NATIVE_TEST_RUN_ID -ne $RunId){throw 'Owned native-test environment required'}
$taskRoot=Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)) ('OBTAINER-NativeTests\'+$RunId)
$taskMarker=Get-Content -LiteralPath (Join-Path $taskRoot '.obtainer-native-test.json') -Raw | ConvertFrom-Json
if($taskMarker.kind -ne 'OBTAINER_NATIVE_TEST' -or $taskMarker.run_id -ne $RunId -or $taskMarker.state -ne 'active'){throw 'Native-test marker mismatch'}
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class IsolatedUpgradeDialog {
  public delegate bool Callback(IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(Callback cb,IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr w,Callback cb,IntPtr p);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr w,StringBuilder b,int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr w,out uint p);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr w);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr w,int id);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr w,int index);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr w,uint m,IntPtr a,IntPtr b);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr w,uint m,IntPtr a,IntPtr b);
}
'@
$script:taskDialog=[IntPtr]::Zero
$taskDeadline=[DateTime]::UtcNow.AddSeconds(30)
$taskProcess=$null
while($script:taskDialog -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $taskDeadline){
    $taskChildren=@(Get-CimInstance Win32_Process -Filter "ParentProcessId=$LauncherProcessId AND Name='obtainer-native-test.exe'")
    if($taskChildren.Count -gt 1){throw 'Ambiguous owned test process'}
    if($taskChildren.Count -eq 1){
        $taskProcess=$taskChildren[0]
        [IsolatedUpgradeDialog]::EnumWindows({param($window,$unused)
            $taskOwner=[uint32]0; [void][IsolatedUpgradeDialog]::GetWindowThreadProcessId($window,[ref]$taskOwner)
            $taskTitle=New-Object Text.StringBuilder 512; [void][IsolatedUpgradeDialog]::GetWindowText($window,$taskTitle,512)
            if($taskOwner -eq $taskProcess.ProcessId -and [IsolatedUpgradeDialog]::IsWindowVisible($window) -and $taskTitle.ToString() -eq 'OBTAINER - upgrade library'){$script:taskDialog=$window}
            return $true
        },[IntPtr]::Zero) | Out-Null
    }
    if($script:taskDialog -eq [IntPtr]::Zero){Start-Sleep -Milliseconds 50}
}
if($script:taskDialog -eq [IntPtr]::Zero){throw 'Owned isolated library dialog not found'}
$script:taskText=''
[IsolatedUpgradeDialog]::EnumChildWindows($script:taskDialog,{param($window,$unused)
    $taskValue=New-Object Text.StringBuilder 8192; [void][IsolatedUpgradeDialog]::GetWindowText($window,$taskValue,8192)
    $script:taskText+=$taskValue.ToString()+"`n"; return $true
},[IntPtr]::Zero) | Out-Null
if(-not $script:taskText.Contains((Join-Path $taskRoot 'data\backups')) -or -not $script:taskText.Contains('one-way')){throw 'Dialog is not the isolated fixture upgrade'}
$taskReadyDeadline=[DateTime]::UtcNow.AddSeconds(5)
do {
    $taskNoButton=[IsolatedUpgradeDialog]::GetDlgItem($script:taskDialog,7)
    $taskNoStyle=[IsolatedUpgradeDialog]::GetWindowLong($taskNoButton,-16)
    $taskDefault=[IsolatedUpgradeDialog]::SendMessage($script:taskDialog,0x0400,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64()
    if(($taskNoStyle -band 15) -eq 1 -and ($taskDefault -band 0xffff) -eq 7){break}
    Start-Sleep -Milliseconds 50
} while([DateTime]::UtcNow -lt $taskReadyDeadline)
if(($taskNoStyle -band 15) -ne 1 -or ($taskDefault -band 0xffff) -ne 7){throw "No is not the default button: style=$taskNoStyle; default=$taskDefault; handle=$taskNoButton"}
$taskChoice=if($Answer -eq 'Yes'){6}else{7}
$taskOwnedProcess=Get-Process -Id $taskProcess.ProcessId
$null=$taskOwnedProcess.Handle
if(-not [IsolatedUpgradeDialog]::PostMessage($script:taskDialog,0x0111,[IntPtr]$taskChoice,[IntPtr]::Zero)){throw 'Could not answer owned fixture dialog'}
$taskExitCode=$null
if($Answer -eq 'No'){
    if(-not $taskOwnedProcess.WaitForExit(15000)){throw 'No did not close the owned fixture process'}
    $taskExitCode=$taskOwnedProcess.ExitCode
    if($taskExitCode -ne 0){throw 'Native No did not exit successfully'}
}
$taskOwnedProcess.Dispose()
[pscustomobject]@{processId=$taskProcess.ProcessId;dialogTitle='OBTAINER - upgrade library';defaultButtonStyle=($taskNoStyle -band 15);defaultButtonId=($taskDefault -band 0xffff);answer=$Answer;exitCode=$taskExitCode;message=$script:taskText} | ConvertTo-Json
