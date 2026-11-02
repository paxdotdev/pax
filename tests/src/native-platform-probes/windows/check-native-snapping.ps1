param([Parameter(Mandatory=$true)][string]$Executable,[Parameter(Mandatory=$true)][string]$OutputDirectory,[ValidateRange(10,180)][int]$Seconds=60,[string]$ExpectedHostSha256='',[string]$ExpectedExecutableSha256='',[string]$ExpectedCartridgeSha256='')
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
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
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
$hostPath=Join-Path (Split-Path $Executable) 'PaxHost.dll';$hostSha256=(Get-FileHash $hostPath).Hash
if($ExpectedHostSha256 -and $hostSha256 -ine $ExpectedHostSha256){throw "Unexpected native host DLL: $hostSha256"}
$exeSha256=(Get-FileHash $Executable).Hash
if($ExpectedExecutableSha256 -and $exeSha256 -ine $ExpectedExecutableSha256){throw "Unexpected fixture executable: $exeSha256"}
$cartridgePath=Join-Path (Split-Path $Executable) 'native_scroll_snap.dll'
if($ExpectedCartridgeSha256 -and (!(Test-Path $cartridgePath) -or (Get-FileHash $cartridgePath).Hash -ine $ExpectedCartridgeSha256)){throw 'Unexpected fixture cartridge'}
$oldLog=$env:PAX_WINDOWS_HOST_LOG;$env:PAX_WINDOWS_HOST_LOG=Join-Path $run 'native-host.log';$process=$null
$oldDpi=[PaxControlsInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$report=[ordered]@{result='running';cleanRetirement=$false;startedUtc=[DateTime]::UtcNow.ToString('o');exe=$Executable;hostPath=$hostPath;hostSha256=$hostSha256;sha256=$exeSha256;checks=@();observations=@();screenshots=@();timingScope='host callbacks/submission/publication, not GPU/display/DCV frames';samples=@();scrolls=@()}
function Write-ProbeReport([string]$path,$value){
 $temporary=$path+'.'+[Guid]::NewGuid().ToString('N')+'.tmp'
 try{
  [IO.File]::WriteAllText($temporary,($value|ConvertTo-Json -Depth 12),[Text.UTF8Encoding]::new($true))
  for($attempt=0;$attempt -lt 6;$attempt++){
   try{if([IO.File]::Exists($path)){[IO.File]::Replace($temporary,$path,[NullString]::Value)}else{[IO.File]::Move($temporary,$path)};return}
   catch{
    $cause=$_.Exception;while($cause.InnerException){$cause=$cause.InnerException}
    $code=$cause.HResult -band 0xffff
    if($attempt -eq 5 -or $cause -isnot [IO.IOException] -or $code -notin @(32,33)){throw}
    Start-Sleep -Milliseconds 40
   }
  }
 }finally{if([IO.File]::Exists($temporary)){[IO.File]::Delete($temporary)}}
}
function Read-ProbeReport([string]$path){
 for($attempt=0;$attempt -lt 6;$attempt++){
  $stream=$null;$reader=$null
  try{
   $stream=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
   $reader=[IO.StreamReader]::new($stream,[Text.Encoding]::UTF8,$true)
   $content=$reader.ReadToEnd();$reader.Dispose();$reader=$null;$stream=$null
   return ($content|ConvertFrom-Json)
  }catch{if($attempt -eq 5){throw};Start-Sleep -Milliseconds 40}
  finally{if($reader){$reader.Dispose()}elseif($stream){$stream.Dispose()}}
 }
}
# Publish a complete snapshot so progress readers cannot see a partial JSON
# document. Readers must permit replacement; persistent sharing failures fail.
function Save { Write-ProbeReport (Join-Path $run 'result.json') $report }
function Read-ProbeLog([string]$path){
 for($attempt=0;$attempt -lt 6;$attempt++){
  $stream=$null;$reader=$null
  try{
   $stream=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
   $reader=[IO.StreamReader]::new($stream,[Text.Encoding]::UTF8,$true)
   return $reader.ReadToEnd()
  }catch{
   $cause=$_.Exception;while($cause.InnerException){$cause=$cause.InnerException}
   if($attempt -eq 5 -or $cause -isnot [IO.IOException] -or ($cause.HResult -band 0xffff) -notin @(32,33)){throw}
   Start-Sleep -Milliseconds 40
  }finally{if($reader){$reader.Dispose()}elseif($stream){$stream.Dispose()}}
 }
}
# Only the terminal close record can establish retirement. An initialization
# snapshot with zero exports must never satisfy a later nonzero close.
function Get-ProbeRetirement([string]$text){
 $receipt=@{passed=$false;closedGpuLine=$null;drainedLine=$null;closedGpu=$null;error=$null}
 $lines=@($text -split "`r?`n");$gpuIndex=-1;$markerIndex=-1;$lastNonempty=-1;$payload=$null;$drained=$false
 for($i=0;$i -lt $lines.Count;$i++){
  if(!$lines[$i].Trim()){continue};$lastNonempty=$i
  if($lines[$i] -match '^\d+\s+closed_gpu=(.*)$'){$gpuIndex=$i;$payload=$Matches[1]}
  if($lines[$i] -match '^\d+\s+closed exports_drained=(true|false)\s*$'){$markerIndex=$i;$drained=$Matches[1] -ceq 'true'}
 }
 if($gpuIndex -lt 0){$receipt.error='Missing closed_gpu record';return $receipt}
 $receipt.closedGpuLine=$gpuIndex+1;$receipt.drainedLine=if($markerIndex -ge 0){$markerIndex+1}else{$null}
 if(!$drained -or $markerIndex -le $gpuIndex -or $markerIndex -ne $lastNonempty){$receipt.error='Missing or nonterminal successful drained marker after final GPU close';return $receipt}
 try{
  $gpu=$payload|ConvertFrom-Json;$receipt.closedGpu=$gpu
  foreach($key in @('allocatedBytes','allocatedImages','pendingBytes','pendingFrames','published','retired')){
   $property=$gpu.PSObject.Properties[$key]
   if($null -eq $property){throw "Missing terminal GPU counter: $key"}
   $value=$property.Value
   if($value -isnot [ValueType] -or $value -is [bool] -or [decimal]$value -lt 0 -or [decimal]$value -ne [Math]::Floor([decimal]$value)){throw "Invalid terminal GPU counter: $key"}
  }
  $closing=$gpu.PSObject.Properties['closing']
  if($null -eq $closing -or $closing.Value -isnot [bool] -or !$closing.Value){throw 'Terminal GPU record is not closing'}
  foreach($key in @('allocatedBytes','allocatedImages','pendingBytes','pendingFrames')){if($gpu.$key -ne 0){throw "Terminal GPU resources remain: $key"}}
  if($gpu.published -ne $gpu.retired){throw 'Terminal GPU published/retired counts differ'}
  $receipt.passed=$true
 }catch{$receipt.error=$_.Exception.Message}
 return $receipt
}
function Read-RetirementEvidence([string]$path){
 $report.cleanRetirement=$false
 try{if(Test-Path $path){$report.retirement=Get-ProbeRetirement (Read-ProbeLog $path);$report.cleanRetirement=$report.retirement.passed}}
 catch{
  # Keep the original input/startup failure while recording a final-log error.
  $report.retirementLogReadError=$_.ToString()
  if($report.result -eq 'passed'){$report.result='failed';$report.error=$report.retirementLogReadError}
 }
 if($report.result -eq 'passed' -and !$report.cleanRetirement){$report.result='failed';$report.error='Native/GPU close did not report complete retirement'}
}
function Check($name,[bool]$ok,$detail){$report.checks+=@{name=$name;passed=$ok;detail=$detail};Save;if(-not $ok){throw "$name failed: $($detail|ConvertTo-Json -Compress -Depth 5)"}}
function Items($type){$c=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$type);$a=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$c);for($i=0;$i -lt $a.Count;$i++){,$a.Item($i)}}
function State {
 $names=@(Items ([System.Windows.Automation.ControlType]::Text)|ForEach-Object{$_.Current.Name}|Where-Object{$_ -match '^x='})
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
function Get-IntegerClickPoint($bounds){
 foreach($edge in @($bounds.Left,$bounds.Top,$bounds.Right,$bounds.Bottom)){if([double]::IsNaN($edge) -or [double]::IsInfinity($edge)){return $null}}
 $minX=[Math]::Ceiling($bounds.Left);$maxX=[Math]::Ceiling($bounds.Right)-1
 $minY=[Math]::Ceiling($bounds.Top);$maxY=[Math]::Ceiling($bounds.Bottom)-1
 if($minX -gt $maxX -or $minY -gt $maxY){return $null}
 # Integer conversion rounds a half pixel and can land on an excluded right
 # or bottom edge. Keep physical input inside the half-open visible rectangle.
 return @{x=[int][Math]::Max($minX,[Math]::Min($maxX,[Math]::Floor(($bounds.Left+$bounds.Right)/2)));y=[int][Math]::Max($minY,[Math]::Min($maxY,[Math]::Floor(($bounds.Top+$bounds.Bottom)/2)))}
}
function Click-Item($item){$point=Get-IntegerClickPoint $item.Current.BoundingRectangle;if($null -eq $point){throw 'Requested item has no integer hit point'};Point $point.x $point.y $true;Settle}
function Button([string]$name){$a=@(Items ([System.Windows.Automation.ControlType]::Button)|Where-Object{$_.Current.Name -ceq $name});if($a.Count -ne 1){throw "Button $name ambiguous"};Click-Item $a[0]}
function Visible-Scroll-Bounds($item){
 $b=$item.Current.BoundingRectangle
 $client=New-Object PaxControlsInput+RECT;$origin=New-Object PaxControlsInput+POINT
 [void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$client);[void][PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$origin)
 $b.Intersect([Windows.Rect]::new($origin.X,$origin.Y,$client.Right,$client.Bottom))
 $ancestor=[System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($item)
 while($null -ne $ancestor -and $ancestor.Current.ProcessId -eq $process.Id){
  if($ancestor.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsScrollPatternAvailableProperty)){$b.Intersect($ancestor.Current.BoundingRectangle)}
  $ancestor=[System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($ancestor)
 }
 return $b
}
function Shot($name){Focus;$b=New-Object PaxControlsInput+RECT;[void][PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$b);$bmp=New-Object Drawing.Bitmap(($b.Right-$b.Left),($b.Bottom-$b.Top));$g=[Drawing.Graphics]::FromImage($bmp);try{$g.CopyFromScreen($b.Left,$b.Top,0,0,$bmp.Size);$bmp.Save((Join-Path $run "$name.png"),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$bmp.Dispose()};$report.screenshots+="$name.png";Save}

$oldTimings=$env:PAX_WINDOWS_TIMINGS;$env:PAX_WINDOWS_TIMINGS='1';$report.scope='Native snapping and Button normal/hover/pressed/dynamic style';Save
function Measure-NativeOffsets($peers){
 $peers=@($peers)
 if($peers.Count -ne 2 -or @($peers|Where-Object{$_.id -notmatch '^pax-node-[0-9]+$' -or $_.class -cne 'ScrollViewer'}).Count -ne 0 -or @($peers.id|Select-Object -Unique).Count -ne 2){throw 'Expected two distinct Pax-owned ScrollViewer peers'}
 $horizontal=@($peers|Where-Object{$_.scrollX -and !$_.scrollY});$vertical=@($peers|Where-Object{$_.scrollY -and !$_.scrollX})
 if($horizontal.Count -ne 1 -or $vertical.Count -ne 1){throw 'Expected one horizontal and one vertical native scroller'}
 $values=@{peers=$peers}
 foreach($axis in @('x','y')){
  $p=if($axis -eq 'x'){$horizontal[0]}else{$vertical[0]};$extent=if($axis -eq 'x'){$p.width}else{$p.height};$percent=if($axis -eq 'x'){$p.percentX}else{$p.percentY};$view=if($axis -eq 'x'){$p.viewX}else{$p.viewY}
  foreach($value in @($extent,$percent,$view)){if([double]::IsNaN($value) -or [double]::IsInfinity($value)){throw 'Nonfinite native scroll geometry'}}
  if($extent -le 0 -or $percent -lt 0 -or $percent -gt 100 -or $view -le 0 -or $view -ge 100){throw 'Invalid native scroll geometry'}
  $values[$axis]=$percent/100*$extent*(100/$view-1)
 }
 return $values
}
function Native-Offsets {
 $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::IsScrollPatternAvailableProperty,$true)
 $all=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition);$peers=@()
 for($i=0;$i -lt $all.Count;$i++){
  $item=$all.Item($i);$current=$item.Current
  if($current.ProcessId -ne $process.Id -or $current.AutomationId -notlike 'pax-node-*'){continue}
  $p=$item.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern).Current;$b=$current.BoundingRectangle
  $peers+=@{id=$current.AutomationId;class=$current.ClassName;scrollX=$p.HorizontallyScrollable;scrollY=$p.VerticallyScrollable;percentX=$p.HorizontalScrollPercent;percentY=$p.VerticalScrollPercent;viewX=$p.HorizontalViewSize;viewY=$p.VerticalViewSize;width=$b.Width;height=$b.Height}
 }
 return (Measure-NativeOffsets $peers)
}
function Synced($name){$s=State;$n=Native-Offsets;Check $name ($n.ContainsKey('x') -and $n.ContainsKey('y') -and [Math]::Abs([double]$s.x-$n.x) -lt 1.1 -and [Math]::Abs([double]$s.y-$n.y) -lt 1.1) @{bound=$s;native=$n};return $s}
function Wheel([bool]$horizontal,[int]$delta){Focus;if($horizontal){Local 410 430}else{Local 940 430};if([PaxControlsInput]::Mouse($(if($horizontal){0x1000}else{0x800}),$delta) -ne 1){throw 'Wheel injection failed'};Start-Sleep -Milliseconds 1600;return State}
function Near([double]$value,[double[]]$anchors){foreach($a in $anchors){if([Math]::Abs($value-$a) -lt 1.1){return $true}};return $false}
function Pixel([int]$x,[int]$y){$p=New-Object PaxControlsInput+POINT;[void][PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$p);$bmp=[Drawing.Bitmap]::new(1,1);$g=[Drawing.Graphics]::FromImage($bmp);try{$g.CopyFromScreen($p.X+$x,$p.Y+$y,0,0,$bmp.Size);$c=$bmp.GetPixel(0,0);return @([int]$c.R,[int]$c.G,[int]$c.B)}finally{$g.Dispose();$bmp.Dispose()}}
function ColorCheck($name,[int[]]$expected,[int]$sampleX=36){$actual=Pixel $sampleX 142;Check $name (@(0..2|Where-Object{[Math]::Abs($actual[$_]-$expected[$_]) -gt 3}).Count -eq 0) @{actual=$actual;expected=$expected}}
try {
 Check 'Interactive session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
 # Time executable launch separately from compilation and post-launch UIA inspection.
 # These are host-observed bounds (200 ms polling), not display presentation times.
 $report.startup=@{launchRequestedUtc=[DateTime]::UtcNow.ToString('o');pollMilliseconds=200;firstWindowObservedMs=$null;firstWindowObservedClass=$null;firstWinUIWindowObservedMs=$null;firstWinUIWindowObservedClass=$null;firstWinUIWindowObservedVisible=$null;firstPublishedFrameObservedMs=$null;firstPublishedFrameObservedUtc=$null}
 $startupWatch=[Diagnostics.Stopwatch]::StartNew();Save
 $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $run 'stdout.log') -RedirectStandardError (Join-Path $run 'stderr.log');$handle=$process.Handle;$report.pid=$process.Id
 $report.startup.processStartedUtc=$process.StartTime.ToUniversalTime().ToString('o')
 $until=[DateTime]::UtcNow.AddSeconds(40)
 do {
  Start-Sleep -Milliseconds 200;$process.Refresh()
  if($process.HasExited){throw 'App exited before first frame'}
  if($process.MainWindowTitle -like 'Pax*stopped:*'){throw $process.MainWindowTitle}
  $windowClass='';$winUIVisible=$false
  if($process.MainWindowHandle -ne [IntPtr]::Zero){
   $observedClass=[Text.StringBuilder]::new(256);[void][PaxControlsInput]::GetClassNameW($process.MainWindowHandle,$observedClass,$observedClass.Capacity);$windowClass=$observedClass.ToString()
   # Preserve the historical process-window metric; a launcher window can
   # precede the actual WinUI application window and first publication.
   if($null -eq $report.startup.firstWindowObservedMs){$report.startup.firstWindowObservedMs=$startupWatch.Elapsed.TotalMilliseconds;$report.startup.firstWindowObservedClass=$windowClass}
   $winUIVisible=$windowClass -ceq 'WinUIDesktopWin32WindowClass' -and [PaxControlsInput]::IsWindowVisible($process.MainWindowHandle)
   if($winUIVisible -and $null -eq $report.startup.firstWinUIWindowObservedMs){$report.startup.firstWinUIWindowObservedMs=$startupWatch.Elapsed.TotalMilliseconds;$report.startup.firstWinUIWindowObservedClass=$windowClass;$report.startup.firstWinUIWindowObservedVisible=$true}
  }
  $published=(Test-Path $env:PAX_WINDOWS_HOST_LOG) -and ((Read-ProbeLog $env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published'))
  if($published -and $null -eq $report.startup.firstPublishedFrameObservedMs){$report.startup.firstPublishedFrameObservedMs=$startupWatch.Elapsed.TotalMilliseconds;$report.startup.firstPublishedFrameObservedUtc=[DateTime]::UtcNow.ToString('o')}
 } while((-not $winUIVisible -or -not $published) -and [DateTime]::UtcNow -lt $until)
 $startupWatch.Stop();Save
 Check 'Actual Pax runtime first frame' ((Read-ProbeLog $env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published')) $process.MainWindowHandle.ToInt64()
 $class=[Text.StringBuilder]::new(256);[void][PaxControlsInput]::GetClassNameW($process.MainWindowHandle,$class,$class.Capacity)
 Check 'Actual native WinUI window' ($class.ToString() -eq 'WinUIDesktopWin32WindowClass') $class.ToString()
 Check 'Visible WinUI startup observation recorded' ($null -ne $report.startup.firstWinUIWindowObservedMs -and $report.startup.firstWinUIWindowObservedVisible -eq $true) $report.startup.firstWinUIWindowObservedMs
 $loadedHost=@($process.Modules|Where-Object{$_.ModuleName -ieq 'PaxHost.dll'})
 Check 'Expected native host is loaded' ($loadedHost.Count -eq 1 -and (Get-FileHash $loadedHost[0].FileName).Hash -ieq $hostSha256) @{loadedPath=if($loadedHost.Count){$loadedHost[0].FileName}else{''};expectedPath=$hostPath;sha256=$hostSha256}
 $report.loadedHostPath=$loadedHost[0].FileName;Save
 if($ExpectedExecutableSha256){Check 'Expected fixture executable is loaded' ((Get-FileHash $process.MainModule.FileName).Hash -ieq $ExpectedExecutableSha256) @{loadedPath=$process.MainModule.FileName;sha256=$exeSha256}}
 $report.loadedCartridge=@($process.Modules|Where-Object{$_.ModuleName -ieq 'native_scroll_snap.dll'}|ForEach-Object{@{path=$_.FileName;sha256=(Get-FileHash $_.FileName).Hash}});Save
 if($ExpectedCartridgeSha256){Check 'Expected fixture cartridge is loaded' ($report.loadedCartridge.Count -eq 1 -and $report.loadedCartridge[0].sha256 -ieq $ExpectedCartridgeSha256) $report.loadedCartridge}
 $scale=[PaxControlsInput]::GetDpiForWindow($process.MainWindowHandle)/96.0;Check '96-DPI exact pixel precondition' ($scale -eq 1) $scale
 [void][PaxControlsInput]::MoveWindow($process.MainWindowHandle,20,20,1040,719,$true);Start-Sleep -Seconds 1
 $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle);Focus;Local 900 600;Check 'Actual desktop input reaches fixture' $true 'strict window ownership and pointer position'
 $s=State;Check 'Page anchors initial state' ($s.mode -eq 'pages' -and [double]$s.x -eq 0 -and [double]$s.y -eq 0) $s
 ColorCheck 'Authored native Button base color' @(42,68,104)
 ColorCheck 'One-pixel outline leaves inner sample at base' @(42,68,104) 26
 Local 70 140;Start-Sleep -Milliseconds 400;ColorCheck 'Native Button hover color' @(63,96,140)
 [void][PaxControlsInput]::Mouse(2);Start-Sleep -Milliseconds 300;ColorCheck 'Native Button pressed color' @(63,96,140);[void][PaxControlsInput]::Mouse(4);Start-Sleep -Milliseconds 400
 Button 'Switch colors';Local 900 600;ColorCheck 'Reactive native Button base color' @(152,53,37)
 ColorCheck 'Reactive three-pixel native outline' @(24,35,52) 26
 Local 70 140;Start-Sleep -Milliseconds 400;ColorCheck 'Reactive native Button hover color' @(190,79,55)
 Local 900 600;Shot 'styled-initial'
 Button 'Reset'
 $s=Wheel $true 480;Check 'Horizontal wheel lands on page anchor' ((Near ([double]$s.x) @(0,450,900)) -and [double]$s.x -gt 0) $s
 $s=Wheel $false -240;Check 'Vertical wheel lands on page anchor' ((Near ([double]$s.y) @(0,240,480)) -and [double]$s.y -gt 0) $s
 Focus;Local 940 430;foreach($delta in @(-120,-120,120,-120)){if([PaxControlsInput]::Mouse(0x800,$delta) -ne 1){throw 'Rapid wheel injection failed'};Start-Sleep -Milliseconds 40};Start-Sleep -Milliseconds 1800;$s=Synced 'Rapid native wheel sequence preserves shared/native agreement';Check 'Rapid wheel settles to native page anchor' (Near ([double]$s.y) @(0,240,480)) $s
 Button 'Pixel anchors';Button 'Jump to 173';Start-Sleep -Milliseconds 800;$s=State
 $s=Synced 'Authored snap reconciles native and shared offsets'
 Check 'Dynamic pixel anchors apply native authored-jump policy' ((Near ([double]$s.x) @(0,125,325,625)) -and (Near ([double]$s.y) @(0,125,325,480))) $s
 Button 'Jump to 173';Start-Sleep -Milliseconds 800;$s=Synced 'Repeated authored jump acknowledges unchanged native snap target'
 Button 'Clear anchors';Button 'Jump to 173';Start-Sleep -Milliseconds 800;$s=State;Check 'Clearing anchors permits exact off-anchor offset' ([Math]::Abs([double]$s.x-173) -lt 1.1 -and [Math]::Abs([double]$s.y-173) -lt 1.1) $s
 $s=Wheel $false -120;Check 'Cleared anchors preserve continuous wheel movement' ([double]$s.y -gt 173 -and -not(Near ([double]$s.y) @(0,240,480))) $s
 Button 'Page anchors';Button 'Resize viewports';Button 'Jump to 173';Start-Sleep -Milliseconds 800;$s=State
 Check 'Resized viewport percentage anchors settle natively' ($s.viewport -eq '360x180' -and (Near ([double]$s.x) @(0,360,720)) -and (Near ([double]$s.y) @(0,180,360))) $s
 $s=Synced 'Resized native and shared offsets agree'
 Shot 'resized-anchors';Check 'Native window responsive after changes' $process.Responding $process.Id;$report.result='passed'
} catch {$report.result='failed';$report.error=$_.ToString();try{$report.nativeAtFailure=Native-Offsets}catch{}} finally {
 [void][PaxControlsInput]::Mouse(4)
 if($process -and -not $process.HasExited){[void]$process.CloseMainWindow();$closed=$process.WaitForExit(15000);$report.close=@{graceful=$closed;exitCode=if($closed){$process.ExitCode}else{$null}};if(-not $closed){$process.Kill();[void]$process.WaitForExit(5000);$report.close.forced=$true;$report.result='failed'}elseif($process.ExitCode -ne 0){$report.result='failed'}}
 Read-RetirementEvidence $env:PAX_WINDOWS_HOST_LOG
 if(-not $report.cleanRetirement){$report.result='failed'};if(!$report.Contains('close') -or !$report.close.graceful -or $report.close.exitCode -ne 0){$report.result='failed'};$env:PAX_WINDOWS_HOST_LOG=$oldLog;$env:PAX_WINDOWS_TIMINGS=$oldTimings;[void][PaxControlsInput]::SetThreadDpiAwarenessContext($oldDpi);$report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save
}
if($report.result -ne 'passed'){throw $report.error}
