param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
# Run in an unlocked interactive desktop under an external deadline: a UIA
# provider failure can block a synchronous query beyond our polling deadlines.
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Windows.Forms,System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PaxSpatialInput {
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct CURSORINFO { public int Size,Flags; public IntPtr Cursor; public POINT Position; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int X,Y; public uint Data,Flags,Time; public IntPtr Extra; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort Key,Scan; public uint Flags,Time; public IntPtr Extra; }
    [StructLayout(LayoutKind.Explicit)] public struct UNION { [FieldOffset(0)] public MOUSEINPUT Mouse; [FieldOffset(0)] public KEYBDINPUT Keyboard; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint Type; public UNION Data; }
    [DllImport("user32.dll",SetLastError=true)] static extern uint SendInput(uint count, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr window, ref POINT point);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr window,int x,int y,int width,int height,bool repaint);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr window,uint flags);
    [DllImport("user32.dll")] public static extern bool GetCursorInfo(ref CURSORINFO info);
    [DllImport("user32.dll")] public static extern IntPtr LoadCursor(IntPtr instance,IntPtr id);
    public static uint Key(ushort key,bool up) {
        INPUT value=new INPUT(); value.Type=1; value.Data.Keyboard.Key=key; value.Data.Keyboard.Flags=up?2u:0u;
        return SendInput(1,new INPUT[]{value},Marshal.SizeOf(typeof(INPUT)));
    }
    public static uint Click() {
        INPUT down=new INPUT(),up=new INPUT(); down.Data.Mouse.Flags=2; up.Data.Mouse.Flags=4;
        return SendInput(2,new INPUT[]{down,up},Marshal.SizeOf(typeof(INPUT)));
    }
}
'@

$Executable=(Resolve-Path -LiteralPath $Executable).Path
$run=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path $run){throw 'Use a fresh evidence directory'}
New-Item -ItemType Directory $run|Out-Null
$report=[ordered]@{passed=$false;executable=$Executable;packageHashes=@{};checks=@();requiresVisualReview=$true;perspectiveQualified=$false}
foreach($name in @('spatial-workbench.exe','spatial_workbench.dll','PaxHost.dll','pax-app.json')){
 $report.packageHashes[$name]=(Get-FileHash (Join-Path (Split-Path $Executable) $name)).Hash
}
$oldLog=$env:PAX_WINDOWS_HOST_LOG;$env:PAX_WINDOWS_HOST_LOG=Join-Path $run 'native-host.log'
$oldDpi=[PaxSpatialInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4));$process=$null
function Save {$report|ConvertTo-Json -Depth 8|Set-Content (Join-Path $run 'result.json') -Encoding UTF8}
function Check($label,[bool]$ok,$detail){$report.checks+=@{name=$label;passed=$ok;detail=$detail};Save;if(!$ok){throw $label}}
function Log {if(Test-Path $env:PAX_WINDOWS_HOST_LOG){return [IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG)};return ''}
function Find($type,$name){
 $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$type)
 $all=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
 for($i=0;$i -lt $all.Count;$i++){$e=$all.Item($i);if($null -eq $name -or $e.Current.Name -ceq $name){return $e}}
 return $null
}
function Button($name){Find ([System.Windows.Automation.ControlType]::Button) $name}
function Wait($predicate,$label){
 $until=[DateTime]::UtcNow.AddSeconds(20)
 do{$process.Refresh();if($process.HasExited){throw 'Fixture exited'};if(& $predicate){return};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $until)
 throw ('Timeout: '+$label)
}
function Point($x,$y){
 Check 'Native mouse placement' ([PaxSpatialInput]::SetCursorPos($x,$y)) @($x,$y)
 Check 'Native mouse delivery' ([PaxSpatialInput]::Click() -eq 2) $null
 Start-Sleep -Milliseconds 120
}
function Click($element){
 if($null -eq $element){throw 'Missing native input target'}
 $b=$element.Current.BoundingRectangle
 Check 'Visible native input target' (!$element.Current.IsOffscreen -and $b.Width -gt 2 -and $b.Height -gt 2) $b.ToString()
 Point ([int]($b.Left+$b.Width/2)) ([int]($b.Top+$b.Height/2))
}
function Value {($editor.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)).Current.Value}
function Capture($name){
 Start-Sleep -Milliseconds 300
 $b=New-Object PaxSpatialInput+RECT
 if(![PaxSpatialInput]::GetWindowRect($process.MainWindowHandle,[ref]$b)){throw 'No owned window rectangle'}
 $bitmap=New-Object Drawing.Bitmap(($b.Right-$b.Left),($b.Bottom-$b.Top));$g=[Drawing.Graphics]::FromImage($bitmap)
 try{$g.CopyFromScreen($b.Left,$b.Top,0,0,$bitmap.Size);$bitmap.Save((Join-Path $run ($name+'.png')),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$bitmap.Dispose()}
}
try{
 Check 'Interactive desktop' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) $null
 $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $run 'app.stdout.log') -RedirectStandardError (Join-Path $run 'app.stderr.log')
 $handle=$process.Handle;$report.pid=$process.Id
 Wait { $process.MainWindowHandle -ne [IntPtr]::Zero -and (Log).Contains('first_frame_published source=pax-runtime') } 'first GPU frame'
 Check 'First GPU frame' $true $null
 $work=[Windows.Forms.Screen]::FromHandle($process.MainWindowHandle).WorkingArea
 Check 'Wide spatial fixture viewport' ($work.Width -ge 1340 -and $work.Height -ge 1010) $work.ToString()
 Check 'Place owned window' ([PaxSpatialInput]::MoveWindow($process.MainWindowHandle,$work.Left+10,$work.Top+10,1300,990,$true)) $null
 [void][PaxSpatialInput]::SetForegroundWindow($process.MainWindowHandle)
 Start-Sleep -Milliseconds 300
 if([PaxSpatialInput]::GetForegroundWindow() -ne $process.MainWindowHandle){Point ($work.Left+100) ($work.Top+25)}
 Check 'Owned window has foreground' ([PaxSpatialInput]::GetForegroundWindow() -eq $process.MainWindowHandle) $null
 $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
 Click (Button 'Reset pose');Click (Button 'Hide amber');Wait { $null -ne (Button 'Show amber') } 'hide amber'
 $editor=Find ([System.Windows.Automation.ControlType]::Edit) $null
 Check 'Native editor exists' ($null -ne $editor) $null
 $runtimeId=$editor.GetRuntimeId() -join ',';$automationId=$editor.Current.AutomationId
 $original=$editor.Current.BoundingRectangle
 Capture 'before-input'
 Click $editor
 Wait {$editor.Current.HasKeyboardFocus} 'native editor focus'
 [Windows.Forms.SendKeys]::SendWait('^a');[Windows.Forms.SendKeys]::SendWait('Pax orthographic')
 Wait {(Value) -ceq 'Pax orthographic'} 'native typing'
 Check 'Transformed editor accepts native text' $true (Value)
 Click (Button 'Press / 0');Wait {$null -ne (Button 'Press / 1')} 'native button event'
 Capture 'orthographic'
 Click (Button 'Zoom out');Wait {$editor.Current.BoundingRectangle.Width -lt $original.Width*.9} 'orthographic zoom'
 Check 'Zoom retains native editor' (($editor.GetRuntimeId() -join ',') -ceq $runtimeId -and (Value) -ceq 'Pax orthographic') $automationId
 Capture 'zoomed'
 Click (Button 'Zoom reset');Wait {[Math]::Abs($editor.Current.BoundingRectangle.Width-$original.Width) -lt 2} 'zoom reset'
 $before=$editor.Current.BoundingRectangle
 Click (Button 'Switch to perspective');Wait {$null -ne (Button 'Switch to orthographic')} 'camera toggle'
 Start-Sleep -Milliseconds 300;$after=$editor.Current.BoundingRectangle
 Check 'Unsupported perspective retains orthographic placement' ([Math]::Abs($before.Left-$after.Left) -lt 2 -and [Math]::Abs($before.Top-$after.Top) -lt 2 -and [Math]::Abs($before.Width-$after.Width) -lt 2) @($before.ToString(),$after.ToString())
 Click (Button 'Switch to orthographic');Click (Button 'Aperture off');Wait {$null -ne (Button 'Aperture on')} 'clip toggle';Capture 'clipped'
 Click (Button 'Aperture on');Wait {$null -ne (Button 'Aperture off')} 'clip recovery'
 Click (Button 'Press / 1');Wait {$null -ne (Button 'Press / 2')} 'restored native input'
 Check 'Clipping retains edited native control' (($editor.GetRuntimeId() -join ',') -ceq $runtimeId -and (Value) -ceq 'Pax orthographic') $null
 Capture 'recovered';$report.passed=$true
}catch{
 $report.error=$_.ToString();$report.passed=$false
 try {if($null -ne $editor){$report.editorValue=Value;$report.editorFocused=$editor.Current.HasKeyboardFocus}}catch{}
 try {if($null -ne $process -and !$process.HasExited){Capture 'failure'}}catch{}
 throw
}finally{
 if($null -ne $process -and !$process.HasExited){$requested=$process.CloseMainWindow();$closed=$process.WaitForExit(15000);if(!$closed){$process.Kill();$process.WaitForExit()};$report.close=@{requested=$requested;graceful=$closed;exit=$process.ExitCode}}
 $logText=Log;$clean=$null -ne $process -and $process.HasExited -and $process.ExitCode -eq 0 -and $logText.Contains('closed exports_drained=true')
 $report.checks+=@{name='Normal shutdown retires GPU exports';passed=$clean}
 if(!$clean){$report.passed=$false}
 $env:PAX_WINDOWS_HOST_LOG=$oldLog;[void][PaxSpatialInput]::SetThreadDpiAwarenessContext($oldDpi);Save
}
if(!$report.passed){throw 'Spatial qualification failed'}
