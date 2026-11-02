param([Parameter(Mandatory=$true)][string]$Executable,[Parameter(Mandatory=$true)][string]$OutputDirectory,[string]$ExpectedHostSha256='',[string]$ExpectedExecutableSha256='',[string]$ExpectedCartridgeSha256='')
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
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr window,System.Text.StringBuilder value,int count);
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
$report=[ordered]@{result='running';cleanRetirement=$false;startedUtc=[DateTime]::UtcNow.ToString('o');exe=$Executable;sha256=(Get-FileHash $Executable).Hash;checks=@();observations=@();screenshots=@();timingScope='host callbacks/submission/publication, not GPU/display/DCV frames';samples=@();scrolls=@()}
function Save { $report|ConvertTo-Json -Depth 12|Set-Content (Join-Path $run 'result.json') -Encoding UTF8 }
function Check($name,[bool]$ok,$detail){$report.checks+=@{name=$name;passed=$ok;detail=$detail};Save;if(-not $ok){throw "$name failed: $($detail|ConvertTo-Json -Compress -Depth 5)"}}
function Items($type){$c=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$type);$a=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$c);for($i=0;$i -lt $a.Count;$i++){,$a.Item($i)}}
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

function Attr($element,$name){$p=$element.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern);$id=[System.Windows.Automation.TextPattern]::$name;return $p.DocumentRange.GetAttributeValue($id)}
function Styles([bool]$alternate){
 $single=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -ceq 'Native text'}|Sort-Object{$_.Current.BoundingRectangle.X})
 $multi=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -like 'A longer first line*'}|Sort-Object{$_.Current.BoundingRectangle.X})
 Check 'Three single-line native swatches' ($single.Count -eq 3) $single.Count
 Check 'Three multiline native swatches' ($multi.Count -eq 3) $multi.Count
 $origin=New-Object PaxControlsInput+POINT;[void][PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$origin)
 for($i=0;$i -lt 3;$i++){
  $b=$single[$i].Current.BoundingRectangle;$x=($b.X-$origin.X)/$scale;$y=($b.Y-$origin.Y)/$scale;$w=$b.Width/$scale;$h=$b.Height/$scale
  $expectedX=24+330*$i+$i/2.0*(310-$w);$expectedY=150+$i/2.0*(110-$h)
  Check "Single swatch $i intrinsic alignment" ([Math]::Abs($x-$expectedX) -lt 2 -and [Math]::Abs($y-$expectedY) -lt 2 -and $w -lt 310 -and $h -lt 110) @{actual=@($x,$y,$w,$h);expected=@($expectedX,$expectedY)}
  $weight=Attr $single[$i] 'FontWeightAttribute';$italic=Attr $single[$i] 'IsItalicAttribute';$underline=Attr $single[$i] 'UnderlineStyleAttribute';$foreground=Attr $single[$i] 'ForegroundColorAttribute'
  $report.observations+=@{alternate=$alternate;swatch=$i;weight=$weight;italic=$italic;underline=$underline;foreground=$foreground}
  Check "Single swatch $i native font weight" ([int]$weight -eq $(if($alternate){700}else{400})) $weight
  Check "Single swatch $i native italic" ([bool]$italic -eq $alternate) $italic
  Check "Single swatch $i native underline" (([int]$underline -ne 0) -eq $alternate) $underline
  $expectedFill=if($alternate){145+36*256+74*65536}else{24+58*256+82*65536}
  Check "Single swatch $i native foreground" ([int]$foreground -eq $expectedFill) @{actual=$foreground;expected=$expectedFill}
  Check "Single swatch $i native family" ((Attr $single[$i] 'FontNameAttribute') -ceq 'Arial') (Attr $single[$i] 'FontNameAttribute')
  $m=$multi[$i];$mb=$m.Current.BoundingRectangle;$mp=$m.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
  # Managed UI Automation returns Rect[], unlike the native COM double array.
  $rects=@($mp.DocumentRange.GetBoundingRectangles()|ForEach-Object{$_.X;$_.Y;$_.Width;$_.Height})
  Check "Multiline swatch $i has two native lines" ($rects.Count -eq 8) $rects
  $anchor=if($i -eq 0){$rects[0]-$rects[4]}elseif($i -eq 1){($rects[0]+$rects[2]/2)-($rects[4]+$rects[6]/2)}else{($rects[0]+$rects[2])-($rects[4]+$rects[6])}
  Check "Multiline swatch $i line alignment" ([Math]::Abs($anchor) -lt 2*$scale) $rects
  $mx=($mb.X-$origin.X)/$scale;$my=($mb.Y-$origin.Y)/$scale;$mw=$mb.Width/$scale;$mh=$mb.Height/$scale
  Check "Multiline swatch $i block alignment" ([Math]::Abs($mx-(24+330*$i+(2-$i)/2.0*(310-$mw))) -lt 2 -and [Math]::Abs($my-(318+(130-$mh)/2)) -lt 2) @{bounds=@($mx,$my,$mw,$mh)}
 }
 $radio=@(Items ([System.Windows.Automation.ControlType]::RadioButton)|Where-Object{$_.Current.Name -like '*radio*'})
 Check 'Three genuine native radio choices' ($radio.Count -eq 3) $radio.Count
 foreach($r in $radio){
  # RadioButton exposes its label as Name, without a child TextPattern peer.
  # Verify the composed ink inside its real native bounds instead.
  $b=$r.Current.BoundingRectangle;$bmp=New-Object Drawing.Bitmap([int][Math]::Ceiling($b.Width),[int][Math]::Ceiling($b.Height));$g=[Drawing.Graphics]::FromImage($bmp)
  $expected=if($alternate){@(255,183,211)}else{@(170,221,250)};$ink=0
  try{$g.CopyFromScreen([int]$b.X,[int]$b.Y,0,0,$bmp.Size);for($y=0;$y -lt $bmp.Height;$y++){for($x=0;$x -lt $bmp.Width;$x++){$p=$bmp.GetPixel($x,$y);if([Math]::Abs($p.R-$expected[0]) -le 3 -and [Math]::Abs($p.G-$expected[1]) -le 3 -and [Math]::Abs($p.B-$expected[2]) -le 3){$ink++}}}}
  finally{$g.Dispose();$bmp.Dispose()}
  Check 'Radio composed ink matches authored color' ($ink -ge 10) @{name=$r.Current.Name;matchingPixels=$ink;expectedRgb=$expected;bounds=$b}
 }
 Save
}
function Popup-And-Focus {
 $combos=@(Items ([System.Windows.Automation.ControlType]::ComboBox));Check 'One genuine styled dropdown' ($combos.Count -eq 1) $combos.Count
 $combo=$combos[0];$expand=$combo.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern);$selection=$combo.GetCurrentPattern([System.Windows.Automation.SelectionPattern]::Pattern)
 $before=@($selection.Current.GetSelection());Check 'Initial native dropdown selection' ($before.Count -eq 1 -and $before[0].Current.Name -ceq 'First option') @($before|ForEach-Object{$_.Current.Name})
 $combo.SetFocus();Tap 0x73
 Check 'Keyboard opens native popup' ($expand.Current.ExpandCollapseState -eq [System.Windows.Automation.ExpandCollapseState]::Expanded) $expand.Current.ExpandCollapseState.ToString()
 Shot 'underlined-dropdown-popup';Tap 0x1B
 Check 'Escape closes native popup' ($expand.Current.ExpandCollapseState -eq [System.Windows.Automation.ExpandCollapseState]::Collapsed) $expand.Current.ExpandCollapseState.ToString()
 $focus=[System.Windows.Automation.AutomationElement]::FocusedElement
 Check 'Escape returns focus to genuine dropdown' ($focus.Current.ProcessId -eq $process.Id -and $focus.Current.AutomationId -eq $combo.Current.AutomationId) @{class=$focus.Current.ClassName;id=$focus.Current.AutomationId}
 $combo.SetFocus();Tap 0x73;Tap 0x23;Tap 0x0D
 $after=@($selection.Current.GetSelection());Check 'Native popup keyboard commits option' ($after.Count -eq 1 -and $after[0].Current.Name -ceq 'Third option') @($after|ForEach-Object{$_.Current.Name})
 $editors=@(Items ([System.Windows.Automation.ControlType]::Edit)|Sort-Object{$_.Current.BoundingRectangle.Y});Check 'Native single and multiline editors remain present' ($editors.Count -eq 2) $editors.Count
 $script:editor=$editors[0];$script:editorPattern=$editor.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern);$script:beforeEdit=$editorPattern.Current.Value
 $editor.SetFocus();Tap 0x23;Tap 0x51
 Check 'Native editor still accepts keyboard input' ($editorPattern.Current.Value -ceq ($beforeEdit+'q')) $editorPattern.Current.Value
 $script:selectedDropdown=$combo
}
try {
 Check 'Interactive session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
 $hostPath=Join-Path (Split-Path $Executable) 'PaxHost.dll';$hostHash=(Get-FileHash $hostPath).Hash;$report.hostSha256=$hostHash
 if($ExpectedHostSha256){Check 'Expected host file hash' ($hostHash -ieq $ExpectedHostSha256) $hostHash}
 if($ExpectedExecutableSha256){Check 'Expected executable file hash' ($report.sha256 -ieq $ExpectedExecutableSha256) $report.sha256}
 $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $run 'stdout.log') -RedirectStandardError (Join-Path $run 'stderr.log');$handle=$process.Handle;$report.pid=$process.Id
 $until=[DateTime]::UtcNow.AddSeconds(60);do{Start-Sleep -Milliseconds 200;$process.Refresh();if($process.HasExited){throw 'App exited before first frame'}}while(($process.MainWindowHandle -eq [IntPtr]::Zero -or -not(Test-Path $env:PAX_WINDOWS_HOST_LOG) -or -not([IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published'))) -and [DateTime]::UtcNow -lt $until)
 Check 'Actual Pax runtime first frame' ([IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published')) $process.MainWindowHandle.ToInt64()
 $report.loadedExecutablePath=$process.MainModule.FileName;$report.loadedExecutableSha256=(Get-FileHash $report.loadedExecutablePath).Hash
 Check 'Expected executable actually running' ($report.loadedExecutablePath -ieq $Executable -and $report.loadedExecutableSha256 -ieq $report.sha256) @{path=$report.loadedExecutablePath;sha256=$report.loadedExecutableSha256}
 $cartridgeName=[IO.Path]::GetFileNameWithoutExtension($Executable).Replace('-','_')+'.dll'
 $cartridges=@($process.Modules|Where-Object{$_.ModuleName -ieq $cartridgeName})
 $report.loadedCartridgePath=if($cartridges.Count -eq 1){$cartridges[0].FileName}else{''}
 $report.cartridgeSha256=if($cartridges.Count -eq 1){(Get-FileHash $cartridges[0].FileName).Hash}else{''}
 if($ExpectedCartridgeSha256){Check 'Expected cartridge actually loaded' ($cartridges.Count -eq 1 -and $report.cartridgeSha256 -ieq $ExpectedCartridgeSha256) @{path=$report.loadedCartridgePath;sha256=$report.cartridgeSha256;expected=$ExpectedCartridgeSha256}}
 $loaded=@($process.Modules|Where-Object{$_.ModuleName -ieq 'PaxHost.dll'});Check 'Expected native DLL loaded' ($loaded.Count -eq 1 -and (Get-FileHash $loaded[0].FileName).Hash -ieq $hostHash) $hostHash
 $class=[Text.StringBuilder]::new(256);[void][PaxControlsInput]::GetClassNameW($process.MainWindowHandle,$class,$class.Capacity);Check 'Actual WinUI window selected' ($class.ToString() -eq 'WinUIDesktopWin32WindowClass') $class.ToString()
 $scale=[PaxControlsInput]::GetDpiForWindow($process.MainWindowHandle)/96.0;$report.dpi=96*$scale
 $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle);Focus;Start-Sleep -Seconds 2;Shot 'initial';Styles $false
 Button 'Change styles';Start-Sleep -Seconds 1;Shot 'alternate';Styles $true;Popup-And-Focus
 Button 'Restore styles';Start-Sleep -Milliseconds 500;Styles $false
 Check 'Reactive style restore preserves native editing state' ($editorPattern.Current.Value -ceq ($beforeEdit+'q')) $editorPattern.Current.Value
 $selected=@($selectedDropdown.GetCurrentPattern([System.Windows.Automation.SelectionPattern]::Pattern).Current.GetSelection());Check 'Reactive style restore preserves committed dropdown binding' ($selected.Count -eq 1 -and $selected[0].Current.Name -ceq 'Third option') @($selected|ForEach-Object{$_.Current.Name})
 $report.result='passed'
}catch{$report.result='failed';$report.error=$_.ToString();throw}finally{
 if($process -and -not $process.HasExited){[void]$process.CloseMainWindow();$closed=$process.WaitForExit(15000);$report.close=@{graceful=$closed;exitCode=if($closed){$process.ExitCode}else{$null}};if(!$closed){$process.Kill();$report.result='failed'}elseif($process.ExitCode -ne 0){$report.result='failed'}}
 if(Test-Path $env:PAX_WINDOWS_HOST_LOG){$tail=[IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG);$report.cleanRetirement=$tail.Contains('closed exports_drained=true') -and $tail.Contains('"allocatedBytes":0')}
 if(!$report.cleanRetirement){$report.result='failed'};$report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save;$env:PAX_WINDOWS_HOST_LOG=$oldLog;[void][PaxControlsInput]::SetThreadDpiAwarenessContext($oldDpi)
}
if($report.result -ne 'passed'){throw $report.error}
