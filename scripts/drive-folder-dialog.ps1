param([Parameter(Mandatory=$true)][string]$SelectedPath,[Parameter(Mandatory=$true)][int]$OwnerPid)
$ErrorActionPreference='Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ObtainerFolderDialog {
 public delegate bool WindowCallback(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumWindows(WindowCallback cb,IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h,WindowCallback cb,IntPtr p);
 [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder s,int count);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h,int msg,IntPtr w,string text);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,int msg,IntPtr w,IntPtr l);
 public static IntPtr Find(uint owner) { IntPtr found=IntPtr.Zero; EnumWindows((h,p)=>{ uint pid;GetWindowThreadProcessId(h,out pid);var cls=new StringBuilder(128);GetClassName(h,cls,128);if(pid==owner&&cls.ToString()=="#32770"&&IsWindowVisible(h)){found=h;return false;}return true;},IntPtr.Zero);return found; }
 public static IntPtr Control(IntPtr parent,string className,int id) { IntPtr found=IntPtr.Zero;EnumChildWindows(parent,(h,p)=>{var cls=new StringBuilder(128);GetClassName(h,cls,128);if(cls.ToString()==className&&(id<0||GetDlgCtrlID(h)==id)&&IsWindowVisible(h)){found=h;return false;}return true;},IntPtr.Zero);return found; }
}
'@
$deadline=[DateTime]::UtcNow.AddSeconds(20)
$handle=[IntPtr]::Zero
while([DateTime]::UtcNow -lt $deadline){$handle=[ObtainerFolderDialog]::Find([uint32]$OwnerPid);if($handle -ne [IntPtr]::Zero){break};Start-Sleep -Milliseconds 100}
if($handle -eq [IntPtr]::Zero){throw 'Owned native folder picker did not appear.'}
# Use the visible folder edit control. WM_SETTEXT is marshalled across processes;
# BFFM_SETSELECTIONW contains a process-local pointer and is unsafe here.
$edit=[IntPtr]::Zero
while([DateTime]::UtcNow -lt $deadline){$edit=[ObtainerFolderDialog]::Control($handle,'Edit',-1);if($edit -ne [IntPtr]::Zero){break};Start-Sleep -Milliseconds 100}
if($edit -eq [IntPtr]::Zero){throw 'Owned native folder edit control did not appear.'}
[void][ObtainerFolderDialog]::SendMessage($edit,0x000C,[IntPtr]::Zero,$SelectedPath)
$button=[ObtainerFolderDialog]::Control($handle,'Button',1)
if($button -eq [IntPtr]::Zero){throw 'Owned native folder confirmation button did not appear.'}
# The picker validates typed paths asynchronously. Posting BM_CLICK while OK is
# disabled can silently do nothing; wait for readiness and prove actual dismissal.
while([DateTime]::UtcNow -lt $deadline -and -not [ObtainerFolderDialog]::IsWindowEnabled($button)){Start-Sleep -Milliseconds 100}
if(-not [ObtainerFolderDialog]::IsWindowEnabled($button)){throw 'Owned native folder confirmation stayed disabled.'}
if(-not [ObtainerFolderDialog]::PostMessage($button,0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)){throw 'Native folder selection could not be confirmed.'}
while([DateTime]::UtcNow -lt $deadline -and [ObtainerFolderDialog]::IsWindow($handle) -and [ObtainerFolderDialog]::IsWindowVisible($handle)){Start-Sleep -Milliseconds 100}
if([ObtainerFolderDialog]::IsWindow($handle) -and [ObtainerFolderDialog]::IsWindowVisible($handle)){throw 'Owned native folder picker did not close after confirmation.'}
Write-Output 'Owned native folder selected; confirmation dialog closed.'
