param([Parameter(Mandatory=$true)][string]$Executable,[Parameter(Mandatory=$true)][string]$OutputDirectory,[switch]$ExpectRounded,[switch]$ExpectOffGrid,[string]$ExpectedHostSha256='',[string]$ExpectedExecutableSha256='',[string]$ExpectedCartridgeSha256='')
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Windows.Forms,System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PaxControlsInput {
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
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int index);
    public static uint Move(int x,int y) {
        INPUT value=new INPUT(); value.Data.Mouse.Flags=0xC001;
        value.Data.Mouse.X=(int)Math.Round((x-GetSystemMetrics(76))*65535.0/(GetSystemMetrics(78)-1));
        value.Data.Mouse.Y=(int)Math.Round((y-GetSystemMetrics(77))*65535.0/(GetSystemMetrics(79)-1));
        return SendInput(1,new INPUT[]{value},Marshal.SizeOf(typeof(INPUT)));
    }
    public static uint Mouse(uint flags, int data=0) {
        INPUT value=new INPUT(); value.Data.Mouse.Flags=flags; value.Data.Mouse.Data=unchecked((uint)data);
        return SendInput(1,new INPUT[]{value},Marshal.SizeOf(typeof(INPUT)));
    }
    public static uint Click() {
        INPUT down=new INPUT(),up=new INPUT(); down.Data.Mouse.Flags=2; up.Data.Mouse.Flags=4;
        return SendInput(2,new INPUT[]{down,up},Marshal.SizeOf(typeof(INPUT)));
    }
}
'@
$Executable=(Resolve-Path $Executable).Path
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ([Guid]::NewGuid().ToString());New-Item -ItemType Directory -Force $run|Out-Null
$oldLog=$env:PAX_WINDOWS_HOST_LOG;$env:PAX_WINDOWS_HOST_LOG=Join-Path $run 'native-host.log';$process=$null
$oldDpi=[PaxControlsInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$report=[ordered]@{result='running';cleanRetirement=$false;startedUtc=[DateTime]::UtcNow.ToString('o');exe=$Executable;sha256=(Get-FileHash $Executable).Hash;checks=@();observations=@();screenshots=@();expectedRounded=[bool]$ExpectRounded}
function Save { $report|ConvertTo-Json -Depth 12|Set-Content (Join-Path $run 'result.json') -Encoding UTF8 }
function Check($name,[bool]$ok,$detail){$report.checks+=@{name=$name;passed=$ok;detail=$detail};Save;if(-not $ok){throw "$name failed: $($detail|ConvertTo-Json -Compress -Depth 5)"}}
function Items($type){$c=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$type);$a=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$c);for($i=0;$i -lt $a.Count;$i++){,$a.Item($i)}}
function State {
 $names=@(Items ([System.Windows.Automation.ControlType]::Text)|ForEach-Object{$_.Current.Name}|Where-Object{$_ -match '^value='})
 if($names.Count -ne 1){throw "Expected one fixture state label, got $($names.Count)"}
 $s=@{text=$names[0]};foreach($pair in ($names[0] -split ' ')){$kv=$pair -split '=';if($kv.Count -eq 2){$s[$kv[0]]=$kv[1]}};return $s
}
function Settle {Start-Sleep -Milliseconds 350}
function Focus {
 [void][PaxControlsInput]::SetForegroundWindow($process.MainWindowHandle);Start-Sleep -Milliseconds 100
 if([PaxControlsInput]::GetForegroundWindow() -ne $process.MainWindowHandle){$r=New-Object PaxControlsInput+RECT;if(-not [PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$r)){throw 'No window'};Point ($r.Left+100) ($r.Top+16) $true}
 Check 'Fixture owns foreground' ([PaxControlsInput]::GetForegroundWindow() -eq $process.MainWindowHandle) $process.Id
}
function Point([int]$x,[int]$y,[bool]$click=$false){
 $p=New-Object PaxControlsInput+POINT;$p.X=$x;$p.Y=$y;$owner=[PaxControlsInput]::GetAncestor([PaxControlsInput]::WindowFromPoint($p),2)
 if($owner -ne $process.MainWindowHandle){throw "Input target is occluded: $x,$y owner=$owner"}
 if([PaxControlsInput]::Move($x,$y) -ne 1){throw 'Absolute pointer movement injection failed'};Start-Sleep -Milliseconds 80;$actual=New-Object PaxControlsInput+POINT
 if(-not [PaxControlsInput]::GetCursorPos([ref]$actual) -or $actual.X -ne $x -or $actual.Y -ne $y){throw 'Pointer did not reach target'}
 if($click -and [PaxControlsInput]::Click() -ne 2){throw 'Click injection failed'};Start-Sleep -Milliseconds 100
}
function Local([double]$x,[double]$y,[bool]$click=$false){$p=New-Object PaxControlsInput+POINT;if(-not [PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$p)){throw 'No client origin'};Point ([int][Math]::Round($p.X+$x*$scale)) ([int][Math]::Round($p.Y+$y*$scale)) $click}
function Tap([int]$key){if([PaxControlsInput]::GetForegroundWindow() -ne $process.MainWindowHandle){throw 'Lost foreground'};if([PaxControlsInput]::Key($key,$false) -ne 1 -or [PaxControlsInput]::Key($key,$true) -ne 1){throw 'Key injection failed'};Settle}
function Click-Item($item){$b=$item.Current.BoundingRectangle;Point ([int]($b.X+$b.Width/2)) ([int]($b.Y+$b.Height/2)) $true;Settle}
function Button([string]$name){$a=@(Items ([System.Windows.Automation.ControlType]::Button)|Where-Object{$_.Current.Name -ceq $name});if($a.Count -ne 1){throw "Button $name ambiguous"};Click-Item $a[0]}
function Shot($name){Focus;$b=New-Object PaxControlsInput+RECT;[void][PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$b);$bmp=New-Object Drawing.Bitmap(($b.Right-$b.Left),($b.Bottom-$b.Top));$g=[Drawing.Graphics]::FromImage($bmp);try{$g.CopyFromScreen($b.Left,$b.Top,0,0,$bmp.Size);$bmp.Save((Join-Path $run "$name.png"),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$bmp.Dispose()};$report.screenshots+="$name.png";Save}
function Pixel([double]$x,[double]$y){
 $origin=New-Object PaxControlsInput+POINT;[void][PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$origin)
 $bmp=New-Object Drawing.Bitmap(1,1);$g=[Drawing.Graphics]::FromImage($bmp)
 try {$g.CopyFromScreen([int]($origin.X+$x*$scale),[int]($origin.Y+$y*$scale),0,0,$bmp.Size);$c=$bmp.GetPixel(0,0);return @($c.R,$c.G,$c.B)}finally{$g.Dispose();$bmp.Dispose()}
}
function Rounded-Pixel {
 $c=Pixel 28 474;Check 'Standalone corner reveals magenta underlay' ([Math]::Abs($c[0]-220) -le 3 -and [Math]::Abs($c[1]-30) -le 3 -and [Math]::Abs($c[2]-180) -le 3) $c
}
function Sliders([double]$value,[int]$count=5){$a=@(Items ([System.Windows.Automation.ControlType]::Slider));Check 'Genuine native slider count' ($a.Count -eq $count) $a.Count;foreach($s in $a){$r=$s.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern).Current;Check 'Every slider peer reflects engine value' ([Math]::Abs($r.Value-$value) -lt 0.001) @{id=$s.Current.AutomationId;value=$r.Value;min=$r.Minimum;max=$r.Maximum;step=$r.SmallChange}}}
try {
 Check 'Interactive session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
 Check 'No competing fixture' (@(Get-Process ([IO.Path]::GetFileNameWithoutExtension($Executable)) -ErrorAction SilentlyContinue).Count -eq 0) $Executable
 if($ExpectedExecutableSha256){Check 'Expected executable file hash' ($report.sha256 -ieq $ExpectedExecutableSha256) $report.sha256}
 $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $run 'stdout.log') -RedirectStandardError (Join-Path $run 'stderr.log');$handle=$process.Handle;$report.pid=$process.Id
 $until=[DateTime]::UtcNow.AddSeconds(60);do{Start-Sleep -Milliseconds 200;$process.Refresh();if($process.HasExited){throw 'App exited before first frame'}}while(($process.MainWindowHandle -eq [IntPtr]::Zero -or -not (Test-Path $env:PAX_WINDOWS_HOST_LOG) -or -not ([IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published'))) -and [DateTime]::UtcNow -lt $until)
 Check 'Actual Pax runtime first frame' ([IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published')) $process.MainWindowHandle.ToInt64()
 $report.loadedExecutablePath=$process.MainModule.FileName;$report.loadedExecutableSha256=(Get-FileHash $report.loadedExecutablePath).Hash
 Check 'Expected executable actually running' ($report.loadedExecutablePath -ieq $Executable -and $report.loadedExecutableSha256 -ieq $report.sha256) @{path=$report.loadedExecutablePath;sha256=$report.loadedExecutableSha256}
 $cartridgeName=[IO.Path]::GetFileNameWithoutExtension($Executable).Replace('-','_')+'.dll'
 $cartridges=@($process.Modules|Where-Object{$_.ModuleName -ieq $cartridgeName})
 $report.loadedCartridgePath=if($cartridges.Count -eq 1){$cartridges[0].FileName}else{''}
 $report.cartridgeSha256=if($cartridges.Count -eq 1){(Get-FileHash $cartridges[0].FileName).Hash}else{''}
 if($ExpectedCartridgeSha256){Check 'Expected cartridge actually loaded' ($cartridges.Count -eq 1 -and $report.cartridgeSha256 -ieq $ExpectedCartridgeSha256) @{path=$report.loadedCartridgePath;sha256=$report.cartridgeSha256;expected=$ExpectedCartridgeSha256}}
 $loadedHost=@($process.Modules|Where-Object{$_.ModuleName -ieq 'PaxHost.dll'})
 $report.loadedHostPath=if($loadedHost.Count -eq 1){$loadedHost[0].FileName}else{''}
 $report.hostSha256=if($loadedHost.Count -eq 1){(Get-FileHash $loadedHost[0].FileName).Hash}else{''}
 Check 'Expected native host module loaded' ($loadedHost.Count -eq 1 -and (!$ExpectedHostSha256 -or $report.hostSha256 -ieq $ExpectedHostSha256)) @{path=$report.loadedHostPath;sha256=$report.hostSha256;expected=$ExpectedHostSha256}
 $work=[Windows.Forms.Screen]::FromHandle($process.MainWindowHandle).WorkingArea;Check 'Desktop fits fixture' ($work.Width -ge 1020 -and $work.Height -ge 860) $work.ToString()
 [void][PaxControlsInput]::MoveWindow($process.MainWindowHandle,$work.Left+20,$work.Top+20,1016,839,$true);$scale=[PaxControlsInput]::GetDpiForWindow($process.MainWindowHandle)/96.0;Check 'Qualified 96DPI desktop' ($scale -eq 1) $scale
 $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle);Focus;Settle
 $s=State;Check 'Initial engine state and no synthetic events' ($s.value -eq '0.50' -and $s.events -eq '0' -and $s.dropdown -eq '1' -and $s.radio -eq '0' -and $s.other -eq '2') $s;Sliders 0.5;Shot 'initial'
 $combos=@(Items ([System.Windows.Automation.ControlType]::ComboBox)|Sort-Object {$_.Current.BoundingRectangle.Y});Check 'Genuine native dropdown count' ($combos.Count -eq 3) $combos.Count
 $slider=@(Items ([System.Windows.Automation.ControlType]::Slider)|Sort-Object {$_.Current.BoundingRectangle.Y})[0];$slider.SetFocus();Tap 0x27
 $s=State;Check 'Native arrow changes one step and emits exactly one event' ($s.value -eq '0.75' -and $s.events -eq '1') $s;Sliders 0.75
 # Drag starts on the observed slider thumb location, then leaves its bounds.
 $b=$slider.Current.BoundingRectangle;$x=[int]($b.X+12+($b.Width-24)*0.6875);$y=[int]($b.Y+16*$scale)
 $report.observations+=@{name='Slider drag start';bounds=$b.ToString();x=$x;y=$y;hit=[System.Windows.Automation.AutomationElement]::FromPoint([System.Windows.Point]::new($x,$y)).Current.ClassName};Save;Shot 'before-drag'
 Point $x $y;Check 'Slider press delivered' ([PaxControlsInput]::Mouse(2) -eq 1) '';Start-Sleep -Milliseconds 150
 try {for($i=1;$i -le 8;$i++){Point ([int]($x+($b.X-16-$x)*$i/8)) ($y+12);[void][PaxControlsInput]::Mouse(1);Start-Sleep -Milliseconds 30}}finally{[void][PaxControlsInput]::Mouse(4)}
 Settle;$s=State;Check 'Pointer capture drag outside slider updates binding on grid' ($s.value -eq '-2.00' -and [int]$s.events -gt 1) $s
 $events=[int]$s.events;Sliders -2
 Button 'Move range';$s=State;Check 'Disjoint programmatic range has no echo' ($s.value -eq '14.00' -and $s.range -eq '10..20' -and $s.step -eq '2.00' -and [int]$s.events -eq $events) $s;Sliders 14
 Button 'Reset';$s=State;Check 'Inverse range reset preserves event count' ($s.value -eq '0.50' -and [int]$s.events -eq $events) $s;Sliders 0.5
 # Native dropdown keyboard selection after its real popup has opened.
 Local 170 266 $true;Shot 'dropdown-popup';Tap 0x24;Tap 0x28;Tap 0x28;Tap 0x0D
 $s=State;Check 'Native dropdown commits Gamma binding' ($s.dropdown -eq '2') $s
 # Click the second item within the first genuine radio group; other group stays checked independently.
 $radios=@(Items ([System.Windows.Automation.ControlType]::RadioButton));Check 'Native radio buttons exposed' ($radios.Count -eq 6) $radios.Count
 $beta=@($radios|Where-Object{$_.Current.Name -ceq 'Beta'});Check 'First group Beta target unique' ($beta.Count -eq 1) $beta.Count;Click-Item $beta[0]
 $s=State;Check 'Radio selection updates only its group' ($s.radio -eq '1' -and $s.other -eq '2') $s
 Button 'Replace options';$s=State;Check 'Options replacement preserves valid selection' ($s.options -eq '2' -and $s.dropdown -eq '1' -and $s.radio -eq '1' -and [int]$s.events -eq $events) $s
 Button 'Empty options';$s=State;Check 'Empty options emits no invalid sentinel' ($s.options -eq '0' -and $s.dropdown -eq '0' -and $s.radio -eq '0' -and [int]$s.events -eq $events) $s
 foreach($combo in @(Items ([System.Windows.Automation.ControlType]::ComboBox))){$selection=$combo.GetCurrentPattern([System.Windows.Automation.SelectionPattern]::Pattern).Current.GetSelection();Check 'Empty native dropdown has no selection' ($selection.Count -eq 0) $selection.Count}
 Button 'Reset';Button 'Toggle controls';Sliders 0.5 1;Button 'Toggle controls';Sliders 0.5 5
 # A native button in a standalone rounded Scroller must not receive a corner click.
 $before=State;Local 28 474 $true;Settle;$after=State;$cornerOk=$after.corner -eq $before.corner
 $report.observations+=@{name='Standalone Scroller corner button exclusion';passed=$cornerOk;before=$before;after=$after};Save
 if($ExpectRounded){Check 'Rounded corner excludes native button' $cornerOk $after;Rounded-Pixel}
 Local 28 474;Check 'Corner wheel delivered' ([PaxControlsInput]::Mouse(0x800,-120) -eq 1) '';Settle;$afterWheel=State
 $wheelOk=$afterWheel.scroll -eq $after.scroll;$report.observations+=@{name='Standalone Scroller corner wheel exclusion';passed=$wheelOk;before=$after;after=$afterWheel};Save
 if($ExpectRounded){Check 'Rounded corner excludes wheel scrolling' $wheelOk $afterWheel}
 Local 430 670;Check 'Inside wheel delivered' ([PaxControlsInput]::Mouse(0x800,-120) -eq 1) '';Settle;$scroll=State;Check 'Inside wheel changes native bound position' ([double]$scroll.scroll -gt [double]$afterWheel.scroll) $scroll
 Button 'Reset';Button 'Cycle radius';$s=State;Check 'Reactive corner radius update' ($s.radius -eq '8') $s;Shot 'radius-eight'
 if($ExpectRounded){
  Button 'Reset';Button 'Toggle viewport';$s=State;Check 'Public zero viewport height applied' ($s.viewport -eq '0') $s
  Button 'Toggle viewport';$s=State;Check 'Positive viewport height restored' ($s.viewport -eq '250') $s
  Rounded-Pixel;$before=State;Local 28 474 $true;Settle;$s=State;Check 'Restored rounded viewport still excludes corner input' ($s.corner -eq $before.corner) $s
 }
 if($ExpectOffGrid){
  Button 'Reset';$before=State;Button 'Off-grid value';$s=State
  Check 'Off-grid authored value is retained without input echo' ($s.value -eq '0.60' -and $s.events -eq $before.events) $s;Sliders 0.6
  $slider=@(Items ([System.Windows.Automation.ControlType]::Slider)|Sort-Object {$_.Current.BoundingRectangle.Y})[0];$slider.SetFocus();Tap 0x27
  $s=State;Check 'Next native key snaps off-grid value once' ($s.value -eq '0.75' -and [int]$s.events -eq [int]$before.events+1) $s;Sliders 0.75
  $before=State;$slider.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern).SetValue(0.76);Settle;$s=State
  Check 'Equivalent normalized accessibility value emits no duplicate event' ($s.value -eq '0.75' -and $s.events -eq $before.events) $s
 }
 Button 'Reset';$combos=@(Items ([System.Windows.Automation.ControlType]::ComboBox)|Sort-Object {$_.Current.BoundingRectangle.X});Click-Item $combos[-1];Shot 'transformed-dropdown-popup';Tap 0x24;Tap 0x0D;$s=State;Check 'Transformed popup commits native selection' ($s.dropdown -eq '0') $s
 $report.result='passed';$report.finalState=State
} catch {$report.result='failed';$report.error=$_.ToString();throw} finally {
 if($process -and -not $process.HasExited){[void]$process.CloseMainWindow();$closed=$process.WaitForExit(15000);$report.close=@{graceful=$closed;exitCode=if($closed){$process.ExitCode}else{$null}};if($closed -and $process.ExitCode -ne 0){$report.result='failed';$report.error='Application exited nonzero after close'};if(-not $closed){$process.Kill();[void]$process.WaitForExit(5000);$report.result='failed';$report.close.forced=$true}}
 if(Test-Path $env:PAX_WINDOWS_HOST_LOG){$tail=[IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG);$report.cleanRetirement=$tail.Contains('closed exports_drained=true') -and $tail.Contains('"allocatedBytes":0')}
 if($report.result -eq 'passed' -and -not $report.cleanRetirement){$report.result='failed';$report.error='Native/GPU close did not report complete retirement'}
 $report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save;$env:PAX_WINDOWS_HOST_LOG=$oldLog;[void][PaxControlsInput]::SetThreadDpiAwarenessContext($oldDpi)
}

if($report.result -ne 'passed'){throw $report.error}
