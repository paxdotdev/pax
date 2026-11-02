param([Parameter(Mandatory=$true)][string]$Executable,[Parameter(Mandatory=$true)][string]$OutputDirectory,[ValidateRange(10,900)][int]$Seconds=60,[string]$ExpectedFonts='',[switch]$AnimationOnly,[switch]$MemoryPhases,[string]$ExpectedHostSha256='',[string]$ExpectedExecutableSha256='',[string]$ExpectedCartridgeSha256='',[ValidateRange(0,4096)][int]$ClientWidth=0,[ValidateRange(0,4096)][int]$ClientHeight=0,[switch]$WebsitePhases,[switch]$OpenSource,[switch]$PreserveStartupViewport,[switch]$StartupSequence,[switch]$SourceOnly,[string]$NativeSelectionExecutable='',[switch]$WebsiteInteractionsOnly,[string]$WebsiteSourceManifest='',[switch]$WebsiteSourcesOnly,[switch]$WebsiteDemosOnly,[switch]$WebsiteRouteOnly,[ValidateSet('none','memory','stationary')][string]$WebsiteMeasurement='none',[ValidateSet('existing','enabled','disabled')][string]$WebsiteDiagnosticsMode='existing',[ValidateRange(1,100)][int]$LaunchOrdinal=1)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
if($WebsitePhases -and ($AnimationOnly -or $MemoryPhases)){throw 'WebsitePhases has its own stationary and traversal phases'}
if($OpenSource -and -not $WebsitePhases){throw 'OpenSource requires WebsitePhases'}
if($SourceOnly -and (-not $OpenSource -or -not $WebsitePhases)){throw 'SourceOnly requires WebsitePhases and OpenSource'}
if($NativeSelectionExecutable -and -not $OpenSource){throw 'NativeSelectionExecutable requires OpenSource'}
if(($WebsiteSourcesOnly -and $WebsiteDemosOnly) -or ($WebsiteRouteOnly -and ($WebsiteSourcesOnly -or $WebsiteDemosOnly))){throw 'Choose at most one restricted website scope'}
if($WebsiteRouteOnly -and !$WebsiteInteractionsOnly){throw 'WebsiteRouteOnly requires WebsiteInteractionsOnly'}
if($WebsiteDemosOnly -and !$WebsiteInteractionsOnly){throw 'WebsiteDemosOnly requires WebsiteInteractionsOnly'}
if($WebsiteSourcesOnly -and !$WebsiteInteractionsOnly){throw 'WebsiteSourcesOnly requires WebsiteInteractionsOnly'}
if($WebsiteInteractionsOnly -and (!$WebsitePhases -or $OpenSource -or $SourceOnly -or !$WebsiteSourceManifest)){throw 'WebsiteInteractionsOnly requires WebsitePhases and WebsiteSourceManifest, without OpenSource or SourceOnly'}
if($WebsiteMeasurement -ne 'none' -and (!$WebsitePhases -or $OpenSource -or $SourceOnly -or $WebsiteInteractionsOnly -or $StartupSequence)){throw 'WebsiteMeasurement requires WebsitePhases without source, interaction, or startup-screenshot modes'}
if($WebsiteDiagnosticsMode -ne 'existing' -and $WebsiteMeasurement -eq 'none'){throw 'Explicit WebsiteDiagnosticsMode requires WebsiteMeasurement'}
if($WebsiteSourceManifest){$WebsiteSourceManifest=(Resolve-Path $WebsiteSourceManifest).Path}
if($NativeSelectionExecutable){$NativeSelectionExecutable=(Resolve-Path $NativeSelectionExecutable).Path}
if($PreserveStartupViewport -and ($ClientWidth -ne 0 -or $ClientHeight -ne 0)){throw 'PreserveStartupViewport cannot be combined with an explicit viewport'}
if(($ClientWidth -eq 0) -ne ($ClientHeight -eq 0)){throw 'Specify both ClientWidth and ClientHeight, or neither'}
# Separate declared measurement plans from the functional acceptance sequence.
function Get-WebsiteMeasurementPlan([string]$mode){
 if($mode -eq 'stationary'){return ,@{name='top-stationary';direction=0;seconds=45}}
 if($mode -eq 'memory'){
  $plan=@(@{name='top-stationary';direction=0;seconds=90})
  for($n=1;$n -le 3;$n++){$plan+=@{name="traversal-$n-down";direction=-1;seconds=90};$plan+=@{name="traversal-$n-up";direction=1;seconds=90}}
  $plan+=@{name='top-after-traversals';direction=0;seconds=90};return $plan
 }
 if($mode -ne 'none'){throw 'Unknown website measurement mode'}
 return @()
}
function Get-WebsiteDiagnosticEnvironment([string]$mode){
 if($mode -eq 'existing'){return @{PAX_WINDOWS_TIMINGS='1'}}
 if($mode -notin @('enabled','disabled')){throw 'Unknown website diagnostics mode'}
 $value=if($mode -eq 'enabled'){'1'}else{'0'}
 return @{PAX_WINDOWS_TIMINGS=$value;PAX_GPU_MEMORY_DIAGNOSTICS=$value;PAX_RENDER_TIMINGS=$value;PAX_PROPERTY_MEMORY_DIAGNOSTICS=$value;PAX_WINDOWS_PHASE_TIMINGS='0'}
}
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
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window,out uint pid);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr window,System.Text.StringBuilder value,int count);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr window,System.Text.StringBuilder value,int count);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
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
# Live diagnostic writers may keep an append handle open. Permit its writes and
# rotation; only sharing/lock violations retry, and all other read errors fail.
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
# Publish a complete snapshot so progress readers cannot see a partial JSON
# document. Readers must permit replacement; persistent sharing failures fail.
function Save { Write-ProbeReport (Join-Path $run 'result.json') $report }
function TraceOffsets {$v=@{};foreach($f in @('stdout.log','stderr.log','native-host.log')){$path=Join-Path $run $f;$v[$f]=if(Test-Path $path){(Get-Item $path).Length}else{0}};return $v}
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
function Get-IntegerClickPoint($bounds){
 foreach($edge in @($bounds.Left,$bounds.Top,$bounds.Right,$bounds.Bottom)){if([double]::IsNaN($edge) -or [double]::IsInfinity($edge)){return $null}}
 $minX=[Math]::Ceiling($bounds.Left);$maxX=[Math]::Ceiling($bounds.Right)-1
 $minY=[Math]::Ceiling($bounds.Top);$maxY=[Math]::Ceiling($bounds.Bottom)-1
 if($minX -gt $maxX -or $minY -gt $maxY){return $null}
 # Integer conversion rounds a half pixel and can land on an excluded right
 # or bottom edge. Keep physical input inside the half-open visible rectangle.
 return @{x=[int][Math]::Max($minX,[Math]::Min($maxX,[Math]::Floor(($bounds.Left+$bounds.Right)/2)));y=[int][Math]::Max($minY,[Math]::Min($maxY,[Math]::Floor(($bounds.Top+$bounds.Bottom)/2)))}
}
function Get-WebsiteMeasurementPoint($bounds){
 $point=Get-IntegerClickPoint $bounds;if($null -eq $point){return $null}
 # Normalize the measured launch's pointer without relying on the preceding
 # application's hover state. This is placement, not an animation pause.
 $desired=[Math]::Floor($bounds.Left+($bounds.Right-$bounds.Left)*.02)
 $point.x=[int][Math]::Max([Math]::Ceiling($bounds.Left),[Math]::Min([Math]::Ceiling($bounds.Right)-1,$desired))
 return $point
}
function Click-Item($item){$point=Get-IntegerClickPoint $item.Current.BoundingRectangle;if($null -eq $point){throw 'Requested item has no integer hit point'};Point $point.x $point.y $true;Settle}
function Button([string]$name){$a=@(Items ([System.Windows.Automation.ControlType]::Button)|Where-Object{$_.Current.Name -ceq $name});if($a.Count -ne 1){throw "Button $name ambiguous"};Click-Item $a[0]}
# ScrollView exposes its template ScrollPresenter peer in the control view.
# Native runtime IDs distinguish those peers without changing app AutomationIds.
function Scroll-Id($item){if($item.Current.AutomationId -like 'pax-node-*'){return $item.Current.AutomationId};return 'uia-'+($item.GetRuntimeId() -join '-')}
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
function Phase-Image($name){
 $foreground=[PaxControlsInput]::GetForegroundWindow();[uint32]$ownerPid=0;[void][PaxControlsInput]::GetWindowThreadProcessId($foreground,[ref]$ownerPid)
 $title=[Text.StringBuilder]::new(256);[void][PaxControlsInput]::GetWindowTextW($foreground,$title,$title.Capacity)
 $owner=Get-Process -Id $ownerPid -ErrorAction SilentlyContinue
 Check 'Memory phase capture owns foreground' ($foreground -eq $process.MainWindowHandle) @{phase=$name;actualWindow=$foreground.ToInt64();expectedWindow=$process.MainWindowHandle.ToInt64();ownerPid=$ownerPid;ownerProcess=if($owner){$owner.ProcessName}else{''};ownerSession=if($owner){$owner.SessionId}else{$null};title=$title.ToString()}
 $b=New-Object PaxControlsInput+RECT;$origin=New-Object PaxControlsInput+POINT
 [void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$b);[void][PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$origin)
 $bitmap=New-Object Drawing.Bitmap($b.Right,$b.Bottom);$graphics=[Drawing.Graphics]::FromImage($bitmap);$path=Join-Path $run "$name.png"
 try{$graphics.CopyFromScreen($origin.X,$origin.Y,0,0,$bitmap.Size);$bitmap.Save($path,[Drawing.Imaging.ImageFormat]::Png)}finally{$graphics.Dispose();$bitmap.Dispose()}
 return (Get-FileHash $path -Algorithm SHA256).Hash
}
if($MemoryPhases -and (!$AnimationOnly -or $Seconds -lt 180)){throw 'Memory phases require AnimationOnly and at least180 seconds'}
$diagnosticChanges=Get-WebsiteDiagnosticEnvironment $WebsiteDiagnosticsMode;$oldDiagnostics=@{}
foreach($key in $diagnosticChanges.Keys){$oldDiagnostics[$key]=[Environment]::GetEnvironmentVariable($key,'Process');[Environment]::SetEnvironmentVariable($key,$diagnosticChanges[$key],'Process')}
if($WebsiteMeasurement -ne 'none'){
 $flags=@{};foreach($key in @('PAX_WINDOWS_TIMINGS','PAX_GPU_MEMORY_DIAGNOSTICS','PAX_RENDER_TIMINGS','PAX_PROPERTY_MEMORY_DIAGNOSTICS','PAX_WINDOWS_PHASE_TIMINGS')){$flags[$key]=[Environment]::GetEnvironmentVariable($key,'Process')}
 $report.websiteMeasurement=@{mode=$WebsiteMeasurement;diagnosticMode=$WebsiteDiagnosticsMode;runId=[IO.Path]::GetFileName($run);launchOrdinal=$LaunchOrdinal;flags=$flags;plan=@(Get-WebsiteMeasurementPlan $WebsiteMeasurement);scope='instrumented process/allocator and host work; no GPU/display/DCV FPS or global animation pause';intervalExclusions='no screenshots, resizing, source drawers, or unrelated input inside measured phases; sampling and report serialization remain active'}
}
$tail='';$report.animationOnly=[bool]$AnimationOnly;$report.clippedScrollerSkips=@()
try {
 Check 'Interactive session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
 Check 'No competing fixture' (@(Get-Process ([IO.Path]::GetFileNameWithoutExtension($Executable)) -ErrorAction SilentlyContinue).Count -eq 0) $Executable
 if($ExpectedExecutableSha256){Check 'Expected executable file hash' ($report.sha256 -ieq $ExpectedExecutableSha256) $report.sha256}
 # Time executable launch separately from compilation and post-launch UIA inspection.
 # These are host-observed bounds (200 ms polling), not display presentation times.
 $report.startup=@{launchRequestedUtc=[DateTime]::UtcNow.ToString('o');pollMilliseconds=200;firstWindowObservedMs=$null;firstWindowObservedClass=$null;firstWinUIWindowObservedMs=$null;firstWinUIWindowObservedClass=$null;firstWinUIWindowObservedVisible=$null;firstPublishedFrameObservedMs=$null;firstPublishedFrameObservedUtc=$null}
 $startupWatch=[Diagnostics.Stopwatch]::StartNew();Save
 $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $run 'stdout.log') -RedirectStandardError (Join-Path $run 'stderr.log');$handle=$process.Handle;$report.pid=$process.Id
 $report.startup.processStartedUtc=$process.StartTime.ToUniversalTime().ToString('o')
 $until=[DateTime]::UtcNow.AddSeconds(60)
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
 Check 'Actual WinUI window selected' ($class.ToString() -eq 'WinUIDesktopWin32WindowClass') $class.ToString()
 Check 'Visible WinUI startup observation recorded' ($null -ne $report.startup.firstWinUIWindowObservedMs -and $report.startup.firstWinUIWindowObservedVisible -eq $true) $report.startup.firstWinUIWindowObservedMs
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

 if($StartupSequence){
  # Capture actual native pixels before the expensive UIA inventory. A first
  # publication can precede native text remeasurement and settled layout.
  $report.startup.visualCheckpoints=@();$report.startup.layoutReadiness='manual review of timed native screenshots; first publication alone is not settled-layout acceptance'
  $firstPublishedUtc=[DateTime]::Parse($report.startup.firstPublishedFrameObservedUtc).ToUniversalTime()
  foreach($secondsAfterFirst in @(0,2,5,10)){
   while(([DateTime]::UtcNow-$firstPublishedUtc).TotalSeconds -lt $secondsAfterFirst){Start-Sleep -Milliseconds 20}
   $process.Refresh();if($process.HasExited){throw 'App exited during startup sequence'}
   if($process.MainWindowTitle -like 'Pax*stopped:*'){throw $process.MainWindowTitle}
   $name="startup-$($secondsAfterFirst)s";Shot $name
   $observed=[DateTime]::UtcNow
   $report.startup.visualCheckpoints+=@{screenshot="$name.png";requestedSecondsAfterFirstPublication=$secondsAfterFirst;observedUtc=$observed.ToString('o');observedMsAfterLaunch=($observed-[DateTime]::Parse($report.startup.launchRequestedUtc).ToUniversalTime()).TotalMilliseconds;observedMsAfterFirstPublication=($observed-$firstPublishedUtc).TotalMilliseconds;traceOffsets=(TraceOffsets)}
   Save
  }
 }

 $work=[Windows.Forms.Screen]::FromHandle($process.MainWindowHandle).WorkingArea
 $scale=[PaxControlsInput]::GetDpiForWindow($process.MainWindowHandle)/96.0;$report.dpi=96*$scale
 $client=New-Object PaxControlsInput+RECT;$outer=New-Object PaxControlsInput+RECT
 [void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$client);[void][PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$outer)
 $report.startup.clientWidthPixels=$client.Right;$report.startup.clientHeightPixels=$client.Bottom
 $width=[Math]::Min(1460,$work.Width-20);$height=[Math]::Min(1000,$work.Height-20)
 if($ClientWidth -gt 0){
  $width=[int][Math]::Round($ClientWidth*$scale)+$outer.Right-$outer.Left-$client.Right
  $height=[int][Math]::Round($ClientHeight*$scale)+$outer.Bottom-$outer.Top-$client.Bottom
  Check 'Requested client viewport fits desktop' ($width -le $work.Width-20 -and $height -le $work.Height-20) @{clientWidth=$ClientWidth;clientHeight=$ClientHeight;scale=$scale;available=$work.ToString()}
 }
 if($PreserveStartupViewport){$width=$outer.Right-$outer.Left;$height=$outer.Bottom-$outer.Top}else{[void][PaxControlsInput]::MoveWindow($process.MainWindowHandle,$work.Left+10,$work.Top+10,$width,$height,$true)}
 $report.preservedStartupViewport=[bool]$PreserveStartupViewport
 Start-Sleep -Seconds 2
 [void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$client)
 $report.clientViewport=@{width=$client.Right/$scale;height=$client.Bottom/$scale;widthPixels=$client.Right;heightPixels=$client.Bottom;requestedWidth=$ClientWidth;requestedHeight=$ClientHeight}
 if($WebsiteMeasurement -ne 'none'){
  $configuration=Join-Path (Split-Path $Executable) 'pax-app.json'
  Check 'Measurement package configuration exists' (Test-Path $configuration) $configuration
  $report.websiteMeasurement.viewport=$report.clientViewport
  $report.websiteMeasurement.packageHashes=@{executable=$report.loadedExecutableSha256;cartridge=$report.cartridgeSha256;host=$report.hostSha256;configuration=(Get-FileHash $configuration).Hash}
  $report.websiteMeasurement.pid=$process.Id;$report.websiteMeasurement.processStartedUtc=$report.startup.processStartedUtc
 }

 if($ClientWidth -gt 0){Check 'Requested client viewport established' ([Math]::Abs($report.clientViewport.width-$ClientWidth) -lt 1 -and [Math]::Abs($report.clientViewport.height-$ClientHeight) -lt 1) $report.clientViewport}
 Save
 $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle);Focus;Settle;Shot 'initial'
 $report.nativeCounts=@{};foreach($type in @([System.Windows.Automation.ControlType]::Button,[System.Windows.Automation.ControlType]::Slider,[System.Windows.Automation.ControlType]::ComboBox,[System.Windows.Automation.ControlType]::RadioButton,[System.Windows.Automation.ControlType]::Edit)){$report.nativeCounts[$type.ProgrammaticName]=@(Items $type).Count}
 $fontNames=@();$reportedFamilies=@();$expectedFamilies=@($ExpectedFonts -split '\|'|Where-Object{$_})
 # Font attributes require cross-process UIA calls; collect only for an explicit font assertion.
 if($expectedFamilies.Count -gt 0){
 foreach($text in @(Items ([System.Windows.Automation.ControlType]::Text))){
  if($text.Current.AutomationId -notlike 'pax-node-*'){continue}
  $pattern=$null
  if(!$text.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern,[ref]$pattern) -or $null -eq $pattern){continue}
  $family=$pattern.DocumentRange.GetAttributeValue([System.Windows.Automation.TextPattern]::FontNameAttribute)
  if($family -is [string]){$fontNames+=,$family}
  $reportedFamilies=@($fontNames|ForEach-Object{($_ -split '#')[-1]})
  if($expectedFamilies.Count -gt 0 -and @($expectedFamilies|Where-Object{$reportedFamilies -notcontains $_}).Count -eq 0){break}
 }
 }
 $report.reportedTextFonts=@($fontNames|Sort-Object -Unique);$report.fontInventoryRequested=($expectedFamilies.Count -gt 0);Save
 foreach($expected in $expectedFamilies){Check 'Native text reports bundled family resource' ($reportedFamilies -contains $expected) @{expected=$expected;actual=$report.reportedTextFonts}}
 $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::IsScrollPatternAvailableProperty,$true)
 $peers=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition);$report.nativeScrollPeers=@(foreach($peer in $peers){$parent=[System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($peer);@{id=$peer.Current.AutomationId;className=$peer.Current.ClassName;name=$peer.Current.Name;parentId=$parent.Current.AutomationId;parentClass=$parent.Current.ClassName;bounds=$peer.Current.BoundingRectangle.ToString()}});Save
 $all=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition);$scrollers=@();for($i=0;$i -lt $all.Count;$i++){$item=$all.Item($i);if($item.Current.AutomationId -like 'pax-node-*' -or $item.Current.AutomationId -eq 'PART_ScrollPresenter'){$scrollers+=,$item}}
 $report.scrollerInventory=@($scrollers|ForEach-Object{$p=$_.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern).Current;@{id=(Scroll-Id $_);automationId=$_.Current.AutomationId;className=$_.Current.ClassName;bounds=$_.Current.BoundingRectangle.ToString();horizontal=$p.HorizontallyScrollable;vertical=$p.VerticallyScrollable;horizontalView=$p.HorizontalViewSize;verticalView=$p.VerticalViewSize}})
 function Scroll-Positions {$allPositions=@{};foreach($scroll in $scrollers){$p=$scroll.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern).Current;$allPositions[(Scroll-Id $scroll)]=@{x=$p.HorizontalScrollPercent;y=$p.VerticalScrollPercent}};return $allPositions}
 if(-not $AnimationOnly){Check 'Canonical fixture exposes native scrollers' ($scrollers.Count -gt 0) $scrollers.Count}
 if($WebsitePhases){
  $report.websitePhases=@();$report.fullWebsiteAcceptance=$false
  $outer=@($scrollers|Where-Object{-not $_.Current.IsOffscreen}|Sort-Object @{Expression={$b=Visible-Scroll-Bounds $_;$b.Width*$b.Height};Descending=$true})|Select-Object -First 1
  Check 'Website outer scroller identified' ($null -ne $outer) $report.scrollerInventory
  $outerPattern=$outer.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
  $report.outerScrollerId=Scroll-Id $outer
  $siteWatch=[Diagnostics.Stopwatch]::StartNew()
  function Website-Sample([string]$phase){
   $sampleStartedUnixMs=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
   $process.Refresh();if($process.HasExited){throw 'Website exited during qualification'}
   if($process.MainWindowTitle -like 'Pax*stopped:*'){throw $process.MainWindowTitle}
   if($process.WorkingSet64 -gt 6GB){throw 'Website exceeded the six-GiB probe safety bound'}
   if($process.PrivateMemorySize64 -gt 8GB){throw ('Website exceeded the eight-GiB private-bytes probe safety bound: '+$process.PrivateMemorySize64)}
   $sample=@{sampleId=$report.samples.Count;pid=$process.Id;sampleStartedUnixMs=$sampleStartedUnixMs;phase=$phase;atSeconds=$siteWatch.Elapsed.TotalSeconds;cpuSeconds=$process.TotalProcessorTime.TotalSeconds;privateBytes=$process.PrivateMemorySize64;workingSet=$process.WorkingSet64;handles=$process.HandleCount;threads=$process.Threads.Count;outerY=$outerPattern.Current.VerticalScrollPercent;traceOffsets=(TraceOffsets)}
   $sample.sampleFinishedUnixMs=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
   $report.samples+=,$sample;return $sample
  }
  function Website-End([int]$direction){$position=$outerPattern.Current.VerticalScrollPercent;return [Math]::Abs($position-$(if($direction -lt 0){100}else{0})) -le .05}
  function Website-Phase([string]$name,[int]$direction=0,[int]$phaseSeconds=$Seconds){
   Focus
   if($WebsiteMeasurement -ne 'none' -and $report.websitePhases.Count -eq 0){
    $bounds=Visible-Scroll-Bounds $outer;$point=Get-WebsiteMeasurementPoint $bounds
    $contained=$null -ne $point -and $point.x -ge $bounds.Left -and $point.x -lt $bounds.Right -and $point.y -ge $bounds.Top -and $point.y -lt $bounds.Bottom
    Check 'Measurement pointer target is inside visible outer viewport' $contained @{point=$point;bounds=$bounds.ToString()}
    Point $point.x $point.y
    $actual=New-Object PaxControlsInput+POINT;$observed=[PaxControlsInput]::GetCursorPos([ref]$actual)
    Check 'Measurement pointer reached declared position' ($observed -and $actual.X -eq $point.x -and $actual.Y -eq $point.y) @{x=$actual.X;y=$actual.Y}
    $report.websiteMeasurement.pointer=@{requested=$point;actual=@{x=$actual.X;y=$actual.Y};visibleOuterBounds=$bounds.ToString();observedUnixMs=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds();scope='deterministic placement only; no hover-free or animation-pause claim'};Save
   }
   $first=Website-Sample "$name-start";$phaseWatch=[Diagnostics.Stopwatch]::StartNew();$batches=0
   while($phaseWatch.Elapsed.TotalSeconds -lt $phaseSeconds){
    if($direction -ne 0){
     if(Website-End $direction){Start-Sleep -Seconds 1;if(Website-End $direction){break}}
     $b=Visible-Scroll-Bounds $outer
     if($b.IsEmpty -or $b.Width -lt 50 -or $b.Height -lt 50){throw 'Website outer scroller is not visibly targetable'}
     # Outside margin avoids embedded previews. UIA observations verify the
     # real wheel reaches the outer viewport; no engine setters are used.
     Point ([int]($b.X+$b.Width*.02)) ([int]($b.Y+$b.Height*.5))
     if([PaxControlsInput]::GetForegroundWindow() -ne $process.MainWindowHandle){throw 'Lost website foreground'}
     for($tick=0;$tick -lt 8;$tick++){if([PaxControlsInput]::Mouse(0x800,$direction*120) -ne 1){throw 'Wheel input incomplete'};Start-Sleep -Milliseconds 25};$batches++
    }
    Start-Sleep -Milliseconds 500;[void](Website-Sample $name);Save
   }
   $last=Website-Sample "$name-end"
   $report.websitePhases+=@{phase=$name;startSampleId=$first.sampleId;endSampleId=$last.sampleId;sampleStartedUnixMs=$first.sampleStartedUnixMs;sampleFinishedUnixMs=$last.sampleFinishedUnixMs;requestedMaximumSeconds=$phaseSeconds;seconds=$last.atSeconds-$first.atSeconds;cpuSeconds=$last.cpuSeconds-$first.cpuSeconds;rssStart=$first.workingSet;rssEnd=$last.workingSet;privateStart=$first.privateBytes;privateEnd=$last.privateBytes;inputBatches=$batches;outerStart=$first.outerY;outerEnd=$last.outerY;traceStart=$first.traceOffsets;traceEnd=$last.traceOffsets}
   Save
   if($direction -ne 0){Check "$name reaches settled endpoint" (Website-End $direction) @{start=$first.outerY;end=$last.outerY;batches=$batches}}
  }
  if($WebsiteMeasurement -ne 'none'){
   Check 'Website measurement begins at top endpoint' (Website-End 1) $outerPattern.Current.VerticalScrollPercent
   foreach($phase in @(Get-WebsiteMeasurementPlan $WebsiteMeasurement)){Website-Phase $phase.name $phase.direction $phase.seconds}
   Check 'Website measurement ends at top endpoint' (Website-End 1) $outerPattern.Current.VerticalScrollPercent
  }elseif(-not ($SourceOnly -or $WebsiteInteractionsOnly)){
  Website-Phase 'top-stationary'
  foreach($traversal in @('first-traversal','repeat-traversal')){
   Website-Phase "$traversal-down" -1;Shot "$traversal-bottom"
   Website-Phase "$traversal-up" 1
  }
  # Resizing and optional drawer interaction occur after the timed phases.
  $outerRect=New-Object PaxControlsInput+RECT;[void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$client);[void][PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$outerRect)
  $narrowWidth=[int][Math]::Round(720*$scale)+$outerRect.Right-$outerRect.Left-$client.Right
  [void][PaxControlsInput]::MoveWindow($process.MainWindowHandle,$work.Left+10,$work.Top+10,$narrowWidth,$height,$true);Start-Sleep -Seconds 3;Shot 'narrow'
  [void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$client);$report.narrowViewport=@{width=$client.Right/$scale;height=$client.Bottom/$scale}
  Check 'Narrow native viewport responds' ($process.Responding -and [Math]::Abs($report.narrowViewport.width-720) -lt 1) $report.narrowViewport
  [void][PaxControlsInput]::MoveWindow($process.MainWindowHandle,$work.Left+10,$work.Top+10,$width,$height,$true);Start-Sleep -Seconds 3
  }else{$report.sourceOnly=[bool]$SourceOnly}
  if($WebsiteInteractionsOnly){. (Join-Path $PSScriptRoot 'website-acceptance.ps1');Invoke-WebsiteAcceptance}
  if($OpenSource){
   $labels=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -ceq 'View Source' -and -not $_.Current.IsOffscreen}|Sort-Object @{Expression={$_.Current.BoundingRectangle.Top}})
   Check 'Visible View Source control exists' ($labels.Count -gt 0) $labels.Count
   $beforeCount=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition).Count
   Click-Item $labels[0];Start-Sleep -Seconds 2;Shot 'source-drawer'
   $afterCount=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition).Count
   Check 'Source drawer adds tab and code scrollers' ($afterCount -ge $beforeCount+2) @{before=$beforeCount;after=$afterCount}
   $documents=@(foreach($item in (Items ([System.Windows.Automation.ControlType]::Text))){
    $pattern=$null
    if($item.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern,[ref]$pattern)){
     $content=$pattern.DocumentRange.GetText(-1)
     if($content.Length -gt 80 -and $content.Contains("`n")){[pscustomobject]@{item=$item;pattern=$pattern;content=$content}}
    }
   })
   Check 'Source drawer exposes genuine multiline native text' ($documents.Count -gt 0) $documents.Count
   $source=$documents|Sort-Object {$_.content.Length} -Descending|Select-Object -First 1
   Check 'Source drawer supports native selection' ($source.pattern.SupportedTextSelection -ne [System.Windows.Automation.SupportedTextSelection]::None) $source.pattern.SupportedTextSelection.ToString()
   $report.sourceDocument=@{characters=$source.content.Length;lines=($source.content -split "`n").Count;automationId=$source.item.Current.AutomationId;prefix=$source.content.Substring(0,[Math]::Min(200,$source.content.Length))}
   function Native-Selection([string]$name,[string]$expected){
    if(!$NativeSelectionExecutable){return}
    $expectedPath=Join-Path $run "$name-expected.txt"
    [IO.File]::WriteAllText($expectedPath,$expected,[Text.UTF8Encoding]::new($false))
    $bounds=@(-1);if($expected.Length -le 4096){$bounds=@(4096,-1)}
    foreach($maxChars in $bounds){
     $result=Join-Path $run "$name-$maxChars.jsonl"
     $arguments=''+$process.Id+' "'+$source.item.Current.AutomationId+'" '+$maxChars+' "'+$result+'" "'+$expectedPath+'"'
     $helper=Start-Process $NativeSelectionExecutable -WindowStyle Hidden -ArgumentList $arguments -PassThru -RedirectStandardOutput "$result.stdout" -RedirectStandardError "$result.stderr";$helperHandle=$helper.Handle
     $finished=$helper.WaitForExit(20000)
     if(!$finished){$helper.Kill();$helper.WaitForExit()}
     $rows=@();if(Test-Path $result){$rows=@(Get-Content $result|ForEach-Object{$_|ConvertFrom-Json})}
     $complete=@($rows|Where-Object{$_.PSObject.Properties['stage'] -and $_.stage -eq 'complete'})
     Check "Native COM MTA $name exact selection ($maxChars)" ($finished -and $helper.ExitCode -eq 0 -and $complete.Count -eq 1 -and $complete[0].matchesExpected) @{exit=$helper.ExitCode;finished=$finished;expectedCharacters=$expected.Length;helperSha256=(Get-FileHash $NativeSelectionExecutable).Hash;result=[IO.Path]::GetFileName($result)}
    }
   }
   # Selected TextPatternRange.GetText crashes the installed UIA client on the
   # long website document. Keep that separate failure recorded and verify the
   # real native selection/copy path through keyboard input instead.
   $report.observations+=@{scope='UIA';detail='Selected-range GetText is excluded after a separately recorded native UIA client access violation; actual pointer selection and keyboard copy are required below.'};Save
   $code=$source.item
   do{$code=[System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($code)}while($null -ne $code -and -not $code.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsScrollPatternAvailableProperty))
   Check 'Source code has its own native scroller' ($null -ne $code -and (Scroll-Id $code) -ne (Scroll-Id $outer)) $(if($code){$code.Current.AutomationId}else{''})
   $codePattern=$code.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
   $codeBounds=Visible-Scroll-Bounds $code
   Check 'Source code scrolls in both axes' ($codePattern.Current.HorizontallyScrollable -and $codePattern.Current.VerticallyScrollable) @{x=$codePattern.Current.HorizontallyScrollable;y=$codePattern.Current.VerticallyScrollable;bounds=$codeBounds.ToString()}
   $outerBefore=$outerPattern.Current.VerticalScrollPercent
   foreach($axis in @('vertical','horizontal')){
    $before=if($axis -eq 'vertical'){$codePattern.Current.VerticalScrollPercent}else{$codePattern.Current.HorizontalScrollPercent}
    Point ([int]($codeBounds.X+$codeBounds.Width*.5)) ([int]($codeBounds.Y+$codeBounds.Height*.5))
    $delta=if($before -lt 99){if($axis -eq 'vertical'){-120}else{120}}else{if($axis -eq 'vertical'){120}else{-120}}
    $flags=if($axis -eq 'vertical'){0x800}else{0x1000}
    for($i=0;$i -lt 3;$i++){if([PaxControlsInput]::Mouse($flags,$delta) -ne 1){throw 'Source code wheel incomplete'};Start-Sleep -Milliseconds 80}
    Start-Sleep -Milliseconds 700
    $after=if($axis -eq 'vertical'){$codePattern.Current.VerticalScrollPercent}else{$codePattern.Current.HorizontalScrollPercent}
    Check "Source code $axis wheel changes native offset" ([Math]::Abs($after-$before) -gt .01) @{before=$before;after=$after}
    Check "Source code $axis wheel preserves outer page position" ([Math]::Abs($outerPattern.Current.VerticalScrollPercent-$outerBefore) -lt .05) @{before=$outerBefore;after=$outerPattern.Current.VerticalScrollPercent}
   }
   $x=[int]($codeBounds.X+30);$y=[int]($codeBounds.Y+55)
   Point $x $y
   if([PaxControlsInput]::Mouse(2) -ne 1){throw 'Source selection press failed'}
   try{Point ($x+120) ($y+22)}finally{[void][PaxControlsInput]::Mouse(4)}
   Start-Sleep -Milliseconds 400
   $clipboardBefore=[Windows.Forms.Clipboard]::GetDataObject()
   try{
    function Control-Key([int]$key){if([PaxControlsInput]::Key(0x11,$false) -ne 1){throw 'Control press failed'};try{Tap $key}finally{[void][PaxControlsInput]::Key(0x11,$true)};Start-Sleep -Milliseconds 200}
    [Windows.Forms.Clipboard]::SetText('__pax_selection_probe__');Control-Key 0x43
    $selection=[Windows.Forms.Clipboard]::GetText()
    Check 'Source supports actual drag selection and keyboard copy' ($selection.Length -gt 0 -and $selection.Length -lt $source.content.Length -and $selection -ne '__pax_selection_probe__' -and $source.content.Replace("`r`n","`n").Contains($selection.Replace("`r`n","`n"))) @{characters=$selection.Length}
    Shot 'source-drawer-drag-selection'
    Native-Selection 'drag' ($selection.Replace("`r`n","`n"))
    Control-Key 0x41;Control-Key 0x43
    $selectedAll=[Windows.Forms.Clipboard]::GetText()
    Check 'Source native select-all copies exact Unicode and lines' ($selectedAll.Replace("`r`n","`n") -ceq $source.content.Replace("`r`n","`n")) @{copiedCharacters=$selectedAll.Length;documentCharacters=$source.content.Length}
    Native-Selection 'select-all' $source.content
   }finally{if($clipboardBefore){[Windows.Forms.Clipboard]::SetDataObject($clipboardBefore,$true)}else{[Windows.Forms.Clipboard]::Clear()}}
   Shot 'source-drawer-select-all'
   $newScrolls=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)|Where-Object{($_.Current.AutomationId -like 'pax-node-*' -or $_.Current.AutomationId -eq 'PART_ScrollPresenter') -and (Scroll-Id $_) -ne (Scroll-Id $code) -and $_.Current.BoundingRectangle.Height -ge 25 -and $_.Current.BoundingRectangle.Height -le 35 -and -not $_.Current.IsOffscreen})
   $tabs=$newScrolls|Sort-Object @{Expression={[Math]::Abs($_.Current.BoundingRectangle.Top-$codeBounds.Top)}}|Select-Object -First 1
   Check 'Source tab strip is native and visible' ($null -ne $tabs) $newScrolls.Count
   $textCondition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Text)
   $tabLabels=@($tabs.FindAll([System.Windows.Automation.TreeScope]::Descendants,$textCondition)|Where-Object{-not $_.Current.IsOffscreen}|Sort-Object @{Expression={$_.Current.BoundingRectangle.Left}})
   Check 'Source has a second authored tab' ($tabLabels.Count -ge 2) @($tabLabels|ForEach-Object{$_.Current.Name})
   Click-Item $tabLabels[1];Start-Sleep -Seconds 1
   $second=$source.pattern.DocumentRange.GetText(-1)
   Check 'Second source tab updates native document' ($second.Length -gt 0 -and $second -cne $source.content) @{characters=$second.Length}
   Shot 'source-second-tab'
   $close=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -ceq 'x' -and -not $_.Current.IsOffscreen}|Sort-Object @{Expression={[Math]::Abs($_.Current.BoundingRectangle.Left-$codeBounds.Right)}})
   Check 'Source drawer close control is visible' ($close.Count -gt 0) $close.Count
   Click-Item $close[0];Start-Sleep -Seconds 2
   $closedCount=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition).Count
   Check 'Closing source retires its native scrollers' ($closedCount -le $beforeCount) @{before=$beforeCount;after=$closedCount}
   Shot 'source-drawer-closed';Save
   $nativeLog=(Read-ProbeLog $env:PAX_WINDOWS_HOST_LOG)
   Check 'Source drawer has no native host failure' ($nativeLog -notmatch '(?m)^\d+ error ') ($nativeLog -split "`r?`n"|Select-Object -Last 5)
  }
 }else{
 $watch=[Diagnostics.Stopwatch]::StartNew();$nextSample=0.0;$iteration=0;$memoryPhase='animated';$phaseStarted=0.0;$phaseImages=@{};$report.phaseImageChecks=@();$report.phaseTransitions=@(@{atSeconds=0;phase=$memoryPhase})
 while($watch.Elapsed.TotalSeconds -lt $Seconds){
  if($process.HasExited){throw 'App exited during scrolling'}
  if(-not $MemoryPhases){Focus}
  if($MemoryPhases -and $watch.Elapsed.TotalSeconds -ge $Seconds/3 -and $memoryPhase -eq 'animated'){
   Focus;Button 'PAUSE ANIMATION';Check 'Pause binding changed native toggle label' (@(Items ([System.Windows.Automation.ControlType]::Button)|Where-Object{$_.Current.Name -ceq 'RESUME ANIMATION'}).Count -eq 1) 'RESUME ANIMATION';$memoryPhase='paused';$phaseStarted=$watch.Elapsed.TotalSeconds;$report.phaseTransitions+=@{atSeconds=$phaseStarted;phase=$memoryPhase;traceOffsets=(TraceOffsets)};Save
  }elseif($MemoryPhases -and $watch.Elapsed.TotalSeconds -ge 2*$Seconds/3 -and $memoryPhase -eq 'paused'){
   Focus;Button 'RESUME ANIMATION';Check 'Resume binding changed native toggle label' (@(Items ([System.Windows.Automation.ControlType]::Button)|Where-Object{$_.Current.Name -ceq 'PAUSE ANIMATION'}).Count -eq 1) 'PAUSE ANIMATION';$memoryPhase='resumed';$phaseStarted=$watch.Elapsed.TotalSeconds;$report.phaseTransitions+=@{atSeconds=$phaseStarted;phase=$memoryPhase;traceOffsets=(TraceOffsets)};Save
  }
  if($MemoryPhases){
   $elapsed=$watch.Elapsed.TotalSeconds-$phaseStarted
   if($elapsed -ge 15 -and !$phaseImages.ContainsKey("$memoryPhase-first")){$phaseImages["$memoryPhase-first"]=Phase-Image "$memoryPhase-first"}
   if($elapsed -ge $Seconds/3-15 -and !$phaseImages.ContainsKey("$memoryPhase-last")){
    $last=Phase-Image "$memoryPhase-last";$phaseImages["$memoryPhase-last"]=$last;$first=$phaseImages["$memoryPhase-first"]
    $same=$first -eq $last;$report.phaseImageChecks+=@{phase=$memoryPhase;first=$first;last=$last;identical=$same;atSeconds=$watch.Elapsed.TotalSeconds}
    Check "Composed pixels reflect $memoryPhase state" ($same -eq ($memoryPhase -eq 'paused')) @{first=$first;last=$last;identical=$same}
   }
  }
  if(-not $AnimationOnly){
  $item=$scrollers[$iteration % $scrollers.Count];$b=Visible-Scroll-Bounds $item
  if(-not $b.IsEmpty -and $b.Width -gt 20 -and $b.Height -gt 20 -and -not $item.Current.IsOffscreen){
   $pattern=$item.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
   $before=@{x=$pattern.Current.HorizontalScrollPercent;y=$pattern.Current.VerticalScrollPercent};$allBefore=Scroll-Positions
   Point ([int]($b.X+$b.Width/2)) ([int]($b.Y+$b.Height/2));$delta=if(($iteration/[Math]::Max(1,$scrollers.Count)) % 8 -lt 4){-120}else{120}
   if([PaxControlsInput]::Mouse(0x800,$delta) -ne 1){throw 'Wheel input incomplete'}
   Start-Sleep -Milliseconds 150
   $after=@{x=$pattern.Current.HorizontalScrollPercent;y=$pattern.Current.VerticalScrollPercent}
   if($report.scrolls.Count -lt 100){$report.scrolls+=@{atSeconds=$watch.Elapsed.TotalSeconds;id=(Scroll-Id $item);bounds=$b.ToString();authoredBounds=$item.Current.BoundingRectangle.ToString();wheel=$delta;before=$before;after=$after;allBefore=$allBefore;allAfter=Scroll-Positions}}
  }elseif($report.clippedScrollerSkips -notcontains (Scroll-Id $item)){$report.clippedScrollerSkips+=,(Scroll-Id $item)}
  }
  if($watch.Elapsed.TotalSeconds -ge $nextSample){$process.Refresh();$report.samples+=@{phase=$memoryPhase;traceOffsets=(TraceOffsets);atSeconds=$watch.Elapsed.TotalSeconds;cpuSeconds=$process.TotalProcessorTime.TotalSeconds;privateBytes=$process.PrivateMemorySize64;workingSet=$process.WorkingSet64;handles=$process.HandleCount;threads=$process.Threads.Count};$nextSample+=5;Save}
  $iteration++;Start-Sleep -Milliseconds 120
 }
 $report.durationSeconds=$watch.Elapsed.TotalSeconds;$report.observationIterations=$iteration;$report.wheelIterations=if($AnimationOnly){0}else{$iteration}
 $changed=@($report.scrolls|Where-Object{$_.before.x -ne $_.after.x -or $_.before.y -ne $_.after.y})
 if(-not $AnimationOnly){Check 'Real wheel changes at least one native viewport' ($changed.Count -gt 0) $changed.Count}
 }
 $process.Refresh();$nativeLog=(Read-ProbeLog $env:PAX_WINDOWS_HOST_LOG)
 Check 'Native host did not report render failure' ($process.MainWindowTitle -notlike 'Pax*stopped:*' -and $nativeLog -notmatch '(?m)^\d+ error ') ($nativeLog -split "`r?`n"|Select-Object -Last 5)
 Check 'Native window remains responsive' $process.Responding $process.Id
 Shot 'after-scroll';$report.result='passed'
} catch {
 $report.result='failed';$report.error=$_.ToString();$failure=$_
 if($process){$process.Refresh();if(-not $process.HasExited -and $process.MainWindowHandle -ne [IntPtr]::Zero){try{Shot 'failure'}catch{$report.failureCaptureError=$_.ToString()}}}
 throw $failure
} finally {
 if($process -and -not $process.HasExited){[void]$process.CloseMainWindow();$closed=$process.WaitForExit(15000);$report.close=@{graceful=$closed;exitCode=if($closed){$process.ExitCode}else{$null}};if($closed -and $process.ExitCode -ne 0){$report.close.error='Application exited nonzero after close';if($report.result -eq 'passed'){$report.error=$report.close.error};$report.result='failed'};if(-not $closed){$process.Kill();[void]$process.WaitForExit(5000);$report.result='failed';$report.close.forced=$true}}
 $tail=''
 try{
  if(Test-Path $env:PAX_WINDOWS_HOST_LOG){$tail=Read-ProbeLog $env:PAX_WINDOWS_HOST_LOG;$report.retirement=Get-ProbeRetirement $tail;$report.cleanRetirement=$report.retirement.passed;$report.timings=@($tail -split "`r?`n"|Where-Object{$_ -match 'host_callback_|host_publication_'})|Select-Object -Last 3}
 }catch{
  # Preserve an earlier startup/input failure while making the read error fatal
  # to an otherwise passing run; the original report must still be published.
  $report.retirementLogReadError=$_.ToString();$report.cleanRetirement=$false
  if($report.result -eq 'passed'){$report.result='failed';$report.error=$report.retirementLogReadError}
 }
 if($report.result -eq 'passed' -and -not $report.cleanRetirement){$report.result='failed';$report.error='Native/GPU close did not report complete retirement'}
 $report.nativeComposition=@($tail -split "`r?`n"|Where-Object{$_ -match 'native_composition '})|Select-Object -Last 1;
 $report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save;$env:PAX_WINDOWS_HOST_LOG=$oldLog;foreach($key in $oldDiagnostics.Keys){[Environment]::SetEnvironmentVariable($key,$oldDiagnostics[$key],'Process')};[void][PaxControlsInput]::SetThreadDpiAwarenessContext($oldDpi)
}

if($report.result -ne 'passed'){throw $report.error}
