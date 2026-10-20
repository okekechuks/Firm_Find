param([Parameter(Mandatory=$true)][string]$DialogTitle,[Parameter(Mandatory=$true)][string]$SelectedPath,[Parameter(Mandatory=$true)][int]$OwnerPid)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ObtainerDialog {
 public delegate bool ChildCallback(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h,ChildCallback callback,IntPtr p);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder text,int count);
 [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
 public static string Children(IntPtr parent) {
  var rows=new StringBuilder();
  EnumChildWindows(parent,(h,p)=>{var cls=new StringBuilder(256);GetClassName(h,cls,256);rows.Append(h.ToInt64()+":"+GetDlgCtrlID(h)+":"+cls+"; ");return true;},IntPtr.Zero);
  return rows.ToString();
 }
 public static IntPtr FindControl(IntPtr parent,int id,string className) {
  IntPtr result=IntPtr.Zero;
  EnumChildWindows(parent,(h,p)=>{var cls=new StringBuilder(256);GetClassName(h,cls,256);if(GetDlgCtrlID(h)==id && cls.ToString()==className){result=h;return false;}return true;},IntPtr.Zero);
  return result;
 }
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h,int id);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr p,IntPtr after,string cls,string title);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h,int msg,IntPtr w,string text);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,int msg,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
}
'@
$deadline=[DateTime]::UtcNow.AddSeconds(20)
$handle=[IntPtr]::Zero
while([DateTime]::UtcNow -lt $deadline){
 $candidate=[ObtainerDialog]::FindWindow('#32770',$DialogTitle)
 if($candidate -ne [IntPtr]::Zero){$dialogPid=0;[void][ObtainerDialog]::GetWindowThreadProcessId($candidate,[ref]$dialogPid);if($dialogPid -eq $OwnerPid){$handle=$candidate;break}}
 Start-Sleep -Milliseconds 100
}
if($handle -eq [IntPtr]::Zero){throw "Owned native file dialog did not appear: $DialogTitle"}
$readyDeadline=[DateTime]::UtcNow.AddSeconds(15)
$filenameHandle=[IntPtr]::Zero
$edit=$null
while([DateTime]::UtcNow -lt $readyDeadline){
 $filenameHandle=[ObtainerDialog]::GetDlgItem($handle,1148)
 if($filenameHandle -eq [IntPtr]::Zero){$filenameHandle=[ObtainerDialog]::FindControl($handle,1001,'Edit')}
 if($filenameHandle -ne [IntPtr]::Zero){break}
 $dialog=[Windows.Automation.AutomationElement]::FromHandle($handle)
 $all=$dialog.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
 foreach($element in $all){
  if($element.Current.ControlType -eq [Windows.Automation.ControlType]::Edit -and ($element.Current.Name -match '^File name' -or $element.Current.AutomationId -eq '1148')){$edit=$element;break}
 }
 if($null -ne $edit){break}
 Start-Sleep -Milliseconds 150
}
if($null -ne $edit){
 $pattern=$null
 if($edit.TryGetCurrentPattern([Windows.Automation.ValuePattern]::Pattern,[ref]$pattern)){
  ([Windows.Automation.ValuePattern]$pattern).SetValue($SelectedPath)
 }else{$filenameHandle=[IntPtr]$edit.Current.NativeWindowHandle}
}
if($filenameHandle -ne [IntPtr]::Zero){
 $combo=[ObtainerDialog]::FindWindowEx($filenameHandle,[IntPtr]::Zero,'ComboBox',$null)
 if($combo -ne [IntPtr]::Zero){$filenameHandle=$combo}
 $editHandle=[ObtainerDialog]::FindWindowEx($filenameHandle,[IntPtr]::Zero,'Edit',$null)
 if($editHandle -ne [IntPtr]::Zero){$filenameHandle=$editHandle}
 [void][ObtainerDialog]::SendMessage($filenameHandle,0x000C,[IntPtr]::Zero,$SelectedPath)
}elseif($null -eq $edit){throw ('Native filename control did not become ready. '+[ObtainerDialog]::Children($handle))}
$button=[ObtainerDialog]::FindControl($handle,1,'Button')
$openDeadline=[DateTime]::UtcNow.AddSeconds(10)
while($button -ne [IntPtr]::Zero -and -not [ObtainerDialog]::IsWindowEnabled($button) -and [DateTime]::UtcNow -lt $openDeadline){Start-Sleep -Milliseconds 50}
if($button -ne [IntPtr]::Zero -and -not [ObtainerDialog]::IsWindowEnabled($button)){throw 'Owned native Open/Save button did not become enabled'}
# Send the owned dialog its default IDOK command; acknowledge closure rather than
# treating a queued child-button click as proof of a completed selection.
$invoked=[ObtainerDialog]::PostMessage($handle,0x0111,[IntPtr]1,$button)
if(-not $invoked){throw 'Native Open/Save could not be invoked'}
$closeDeadline=[DateTime]::UtcNow.AddSeconds(10)
while([ObtainerDialog]::IsWindow($handle) -and [DateTime]::UtcNow -lt $closeDeadline){Start-Sleep -Milliseconds 50}
if([ObtainerDialog]::IsWindow($handle)){throw ('Owned native file dialog remained open after IDOK. '+[ObtainerDialog]::Children($handle))}
Write-Output 'Native filename entered and owned Open/Save dialog closed.'
