param([Parameter(Mandatory=$true)][string]$Executable,[Parameter(Mandatory=$true)][string]$OutputDirectory,[Parameter(Mandatory=$true)][string]$ExpectedHostSha256,[Parameter(Mandatory=$true)][string]$ExpectedCartridgeSha256,[Parameter(Mandatory=$true)][string]$NativeUiaExecutable,[Parameter(Mandatory=$true)][string]$ExpectedNativeUiaSha256,[ValidateSet('Debug','Release')][string]$Profile='Release',[string]$ExpectedExecutableSha256='',[uint32]$ExpectedDpi=0)
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
if([Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0){throw 'Interactive desktop required'}
if(@(Get-Process pax-website,native-wheel,native-scroll-snap,pax-cli,cargo,rustc,cl,link,MSBuild -ErrorAction SilentlyContinue).Count){throw 'Competing test/build'}
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
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
$Executable=(Resolve-Path $Executable).Path;$NativeUiaExecutable=(Resolve-Path $NativeUiaExecutable).Path
if($ExpectedExecutableSha256 -and (Get-FileHash $Executable).Hash -ine $ExpectedExecutableSha256){throw 'Executable identity mismatch'}
if((Get-FileHash $NativeUiaExecutable).Hash -ine $ExpectedNativeUiaSha256){throw 'Native observer identity mismatch'}
$run=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path $run){throw 'Preserve earlier fixture run'};New-Item -ItemType Directory $run|Out-Null
$oldEnvironment=@{};foreach($name in @('PAX_WINDOWS_HOST_LOG','PAX_WINDOWS_TIMINGS','PAX_NATIVE_WHEEL_LOG')){$oldEnvironment[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
$env:PAX_WINDOWS_HOST_LOG="$run\native-host.log";$env:PAX_WINDOWS_TIMINGS='1';$env:PAX_NATIVE_WHEEL_LOG="$run\snapshots.log"
$report=[ordered]@{result='running';error=$null;profile=$Profile;cleanRetirement=$false;cleanupErrors=@();startedUtc=[DateTime]::UtcNow.ToString('o');exe=$Executable;exeSha256=(Get-FileHash $Executable).Hash;checks=@();cases=@();nativeObserver=@{path=$NativeUiaExecutable;sha256=(Get-FileHash $NativeUiaExecutable).Hash};nativeObservations=@();scope=('Packaged '+$Profile+' (profile provenance supplied by build receipt), real SendInput mouse wheel, native COM offsets; precision touchpad not qualified')}
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
function Save{Write-ProbeReport "$run\result.json" $report}
# Only complete newline-terminated Rust snapshots can advance the input oracle.
function Get-WheelSnapshotLines([string]$text){
 foreach($match in [regex]::Matches($text,'[^\r\n]+\r?\n')){$match.Value.TrimEnd([char[]]"`r`n")}
}
function Test-WheelReadiness([long]$hwnd,[string]$class,[bool]$visible,[uint32]$ownerPid,[uint32]$expectedPid,[bool]$published){
 return $hwnd -ne 0 -and $class -ceq 'WinUIDesktopWin32WindowClass' -and $visible -and $expectedPid -gt 0 -and $ownerPid -eq $expectedPid -and $published
}
# Close only the Process object created by this checker, including startup failures.
# This path never synthesizes input to make an unready window qualify.
function Close-WheelProcess($owned){
 if($null -eq $owned){return}
 if(!$owned.HasExited){[void]$owned.CloseMainWindow();if(!$owned.WaitForExit(15000)){$owned.Kill();[void]$owned.WaitForExit(5000);$report.forcedAfterCloseTimeout=$true}}
 $report.close=@{graceful=(!$report.Contains('forcedAfterCloseTimeout') -and $owned.HasExited);exitCode=if($owned.HasExited){$owned.ExitCode}else{$null}}
}
function Check([string]$name,[bool]$ok,$detail){$report.checks+=@{name=$name;passed=$ok;detail=$detail};Save;if(!$ok){throw "$name failed"}}
$oldDpi=[PaxControlsInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4));$process=$null;$window=[IntPtr]::Zero;$scale=1.0;$script:observationNumber=0;$script:shiftDown=$false
function Point([double]$x,[double]$y,[bool]$click=$false){
 $p=New-Object PaxControlsInput+POINT;if(![PaxControlsInput]::ClientToScreen($window,[ref]$p)){throw 'No fixture client origin'};$p.X+=[int][Math]::Round($x*$scale);$p.Y+=[int][Math]::Round($y*$scale)
 if([PaxControlsInput]::GetAncestor([PaxControlsInput]::WindowFromPoint($p),2) -ne $window){throw "Input target not owned: $x,$y"}
 if([PaxControlsInput]::Move($p.X,$p.Y) -ne 1){throw 'Pointer move failed'};Start-Sleep -Milliseconds 100
 $actual=New-Object PaxControlsInput+POINT;if(![PaxControlsInput]::GetCursorPos([ref]$actual) -or $actual.X -ne $p.X -or $actual.Y -ne $p.Y){throw 'Pointer not at target'}
 if($click -and [PaxControlsInput]::Click() -ne 2){throw 'Click failed'};Start-Sleep -Milliseconds 100
}
function Observe-Native {
 $script:observationNumber++;$stem=('native-observation-{0:D3}' -f $script:observationNumber);$file=Join-Path $run ($stem+'.jsonl')
 $observer=Start-Process $NativeUiaExecutable -ArgumentList @($process.Id,$window.ToInt64(),('"'+$file+'"')) -PassThru -NoNewWindow -RedirectStandardOutput "$run\$stem.stdout" -RedirectStandardError "$run\$stem.stderr";$observerHandle=$observer.Handle
 if(!$observer.WaitForExit(20000)){$observer.Kill();[void]$observer.WaitForExit(5000);throw ('Native UIA observation exceeded 20s: '+$file)}
 $report.nativeObservations+=@{path=$file;exitCode=$observer.ExitCode};Save
 if($observer.ExitCode -ne 0){throw ('Native UIA observation failed: '+$file)}
 $rows=@([IO.File]::ReadAllLines($file)|ForEach-Object{$_|ConvertFrom-Json});$complete=@($rows|Where-Object{($_.PSObject.Properties.Name -contains 'stage') -and $_.stage -eq 'complete'})
 if($complete.Count -ne 1 -or !$complete[0].readOnly -or $complete[0].pid -ne $process.Id){throw ('Native UIA observation identity missing: '+$file)}
 return $complete[0]
}
function Native-State {
 $found=@();$observation=Observe-Native
 foreach($item in @($observation.elements|Where-Object{$_.scroll})){
  $kind=if([Math]::Abs($item.width/$scale-760) -lt 3){'outer'}elseif([Math]::Abs($item.width/$scale-440) -lt 3){'inner'}else{continue}
  $extentX=if($kind -eq 'inner'){940.0}else{1360.0};$extentY=if($kind -eq 'inner'){660.0}else{900.0}
  $offsetX=if($item.percentX -lt 0){0}else{$extentX*(1-$item.viewX/100)*$item.percentX/100}
  $offsetY=if($item.percentY -lt 0){0}else{$extentY*(1-$item.viewY/100)*$item.percentY/100}
  $found+=@{kind=$kind;id=$item.id;class=$item.class;bounds=('{0},{1},{2},{3}' -f $item.left,$item.top,$item.width,$item.height);x=$offsetX;y=$offsetY;percentX=$item.percentX;percentY=$item.percentY;viewX=$item.viewX;viewY=$item.viewY}
 }
 return ,$found
}
function Snapshot {
 $count=if(Test-Path $env:PAX_NATIVE_WHEEL_LOG){@(Get-WheelSnapshotLines (Read-ProbeLog $env:PAX_NATIVE_WHEEL_LOG)).Count}else{0}
 Point 894 123 $true;$until=[DateTime]::UtcNow.AddSeconds(4)
 do{Start-Sleep -Milliseconds 100;$lines=@(if(Test-Path $env:PAX_NATIVE_WHEEL_LOG){Get-WheelSnapshotLines (Read-ProbeLog $env:PAX_NATIVE_WHEEL_LOG)});if($lines.Count -gt $count){break}}while([DateTime]::UtcNow -lt $until)
 if($lines.Count -le $count){throw 'Fixture snapshot click produced no log'}
 $line=$lines[-1];$v=@{raw=$line}
 foreach($name in @('inner_events','outer_events','root_events','overlay_events','inner_x','inner_y','outer_x','outer_y')){if($line -notmatch ($name+'=(-?[0-9.]+)')){throw 'Malformed fixture state'};$v[$name]=[double]::Parse($matches[1],[Globalization.CultureInfo]::InvariantCulture)}
 $v.prevented=$line.Contains('prevent_native_default=true');$v.native=Native-State;return $v
}
function Reset {
 Point 695 123 $true;Start-Sleep -Milliseconds 700;$v=Snapshot
 Check 'Reset counters and bound native positions settled' (($v.inner_events+$v.outer_events+$v.root_events+$v.overlay_events) -eq 0 -and [Math]::Abs($v.inner_x)+[Math]::Abs($v.inner_y)+[Math]::Abs($v.outer_x)+[Math]::Abs($v.outer_y) -lt 0.05 -and !$v.prevented) $v
 return $v
}
function Screenshot([string]$name){$b=New-Object PaxControlsInput+RECT;[void][PaxControlsInput]::GetWindowRect($window,[ref]$b);$bmp=New-Object Drawing.Bitmap(($b.Right-$b.Left),($b.Bottom-$b.Top));$g=[Drawing.Graphics]::FromImage($bmp);try{$g.CopyFromScreen($b.Left,$b.Top,0,0,$bmp.Size);$bmp.Save("$run\$name.png",[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$bmp.Dispose()}}
try{
 $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput "$run\stdout.log" -RedirectStandardError "$run\stderr.log";$handle=$process.Handle
 $until=[DateTime]::UtcNow.AddSeconds(50);$ready=$false
 $report.startup=@{firstWindowClass=$null;selectedWindow=$null;pollMilliseconds=200}
 do{
  Start-Sleep -Milliseconds 200;$process.Refresh();if($process.HasExited){throw 'Fixture exited before readiness'}
  $candidate=$process.MainWindowHandle;$class=[Text.StringBuilder]::new(256);$ownerPid=[uint32]0
  [void][PaxControlsInput]::GetClassNameW($candidate,$class,$class.Capacity);[void][PaxControlsInput]::GetWindowThreadProcessId($candidate,[ref]$ownerPid)
  if($candidate -ne [IntPtr]::Zero -and $null -eq $report.startup.firstWindowClass){$report.startup.firstWindowClass=$class.ToString()}
  $published=(Test-Path $env:PAX_WINDOWS_HOST_LOG) -and (Read-ProbeLog $env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published')
  $ready=Test-WheelReadiness ($candidate.ToInt64()) ($class.ToString()) ([PaxControlsInput]::IsWindowVisible($candidate)) $ownerPid $process.Id $published
  if($ready){$window=$candidate;$report.startup.selectedWindow=@{hwnd=$window.ToInt64();class=$class.ToString();ownerPid=$ownerPid;visible=$true}}
 }while(!$ready -and [DateTime]::UtcNow -lt $until)
 Check 'Runtime published first frame' $ready $process.Id
 $report.loadedExecutable=@{path=$process.MainModule.FileName;sha256=(Get-FileHash $process.MainModule.FileName).Hash}
 if($report.loadedExecutable.path -ine $Executable -or $report.loadedExecutable.sha256 -ine $report.exeSha256){throw 'Loaded executable differs from requested immutable package'}
 $report.dpi=[PaxControlsInput]::GetDpiForWindow($window)
 if($ExpectedDpi -gt 0 -and $report.dpi -ne $ExpectedDpi){throw 'Fixture DPI differs from qualification scope'}
 [void][PaxControlsInput]::SetForegroundWindow($window);Start-Sleep -Milliseconds 100
 if([PaxControlsInput]::GetForegroundWindow() -ne $window){$b=New-Object PaxControlsInput+RECT;[void][PaxControlsInput]::GetWindowRect($window,[ref]$b);$p=New-Object PaxControlsInput+POINT;$p.X=$b.Left+150;$p.Y=$b.Top+14;if([PaxControlsInput]::GetAncestor([PaxControlsInput]::WindowFromPoint($p),2) -ne $window){throw 'Caption activation not owned'};if([PaxControlsInput]::Move($p.X,$p.Y) -ne 1 -or [PaxControlsInput]::Click() -ne 2){throw 'Caption activation failed'};Start-Sleep -Milliseconds 200}
 Check 'Foreground fixture owned' ([PaxControlsInput]::GetForegroundWindow() -eq $window) $process.Id
 $scale=[PaxControlsInput]::GetDpiForWindow($window)/96.0;$client=New-Object PaxControlsInput+RECT;[void][PaxControlsInput]::GetClientRect($window,[ref]$client)
 Check 'Fixture logical client viewport' ([Math]::Abs($client.Right/$scale-1024) -lt 1 -and [Math]::Abs($client.Bottom/$scale-780) -lt 1) @{width=$client.Right/$scale;height=$client.Bottom/$scale;scale=$scale}
 $hostModule=@($process.Modules|Where-Object{$_.ModuleName -ieq 'PaxHost.dll'});Check 'Qualified candidate host loaded' ($hostModule.Count -eq 1 -and (Get-FileHash $hostModule[0].FileName).Hash -ieq $ExpectedHostSha256) @($hostModule|ForEach-Object{$_.FileName})
 $report.cartridge=@($process.Modules|Where-Object{$_.ModuleName -ieq 'native_wheel.dll'}|ForEach-Object{@{path=$_.FileName;sha256=(Get-FileHash $_.FileName).Hash}});Check 'Genuine fixture cartridge loaded' ($report.cartridge.Count -eq 1 -and $report.cartridge[0].sha256 -ieq $ExpectedCartridgeSha256) $report.cartridge
 Start-Sleep -Milliseconds 600;Screenshot 'initial'
 $cases=@(
  @{name='inner-blank';x=360;y=565;path=@(1,1,1,0);owner='inner';axis='y'},
  @{name='inner-painted-stroke';x=375;y=585;path=@(1,1,1,0);owner='inner';axis='y'},
  @{name='inner-horizontal';x=250;y=500;path=@(1,1,1,0);owner='inner';axis='x'},
  @{name='outer-only';x=720;y=580;path=@(0,1,1,0);owner='outer';axis='y'},
  @{name='fixed-overlay-inner';x=460;y=450;path=@(0,0,1,1);owner='none';axis='y'},
  @{name='fixed-overlay-outer';x=600;y=450;path=@(0,0,1,1);owner='none';axis='y'},
  @{name='vector-overlay-inner';x=418;y=475;path=@(0,0,1,1);owner='none';axis='y';vectorOnly=$true},
  @{name='vector-overlay-outer';x=622;y=475;path=@(0,0,1,1);owner='none';axis='y';vectorOnly=$true},
  @{name='inner-rounded-outside';x=108;y=364;path=@(0,1,1,0);owner='outer';axis='y'},
  @{name='outer-rounded-outside';x=28;y=264;path=@(0,0,1,0);owner='none';axis='y'},
  @{name='cancel-inner';x=250;y=500;path=@(1,1,1,0);owner='none';axis='y';cancel=$true},
  @{name='cancel-painted-stroke';x=375;y=585;path=@(1,1,1,0);owner='none';axis='y';cancel=$true},
  @{name='cancel-outer';x=720;y=580;path=@(0,1,1,0);owner='none';axis='y';cancel=$true},
  @{name='shift-inner';x=250;y=500;path=@(1,1,1,0);owner='inner';axis='y';shift=$true})
 foreach($case in $cases){
  $before=Reset;if($case.ContainsKey('cancel')){Point 38 124 $true;Start-Sleep -Milliseconds 250}
  if($case.ContainsKey('vectorOnly')){
   $observation=Observe-Native;$labels=@($observation.elements|Where-Object{$_.name -ceq 'ROOT OVERLAY · No viewport owns this input'});$rects=@();$outside=$labels.Count -gt 0
   $origin=New-Object PaxControlsInput+POINT;[void][PaxControlsInput]::ClientToScreen($window,[ref]$origin)
   $targetX=$origin.X+$case.x*$scale;$targetY=$origin.Y+$case.y*$scale
   foreach($bounds in $labels){
    $valid=$bounds.width -gt 0 -and $bounds.height -gt 0
    $contains=$valid -and $targetX -ge $bounds.left -and $targetX -lt ($bounds.left+$bounds.width) -and $targetY -ge $bounds.top -and $targetY -lt ($bounds.top+$bounds.height)
    $outside=$outside -and $valid -and !$contains;$rects+=@{left=$bounds.left;top=$bounds.top;width=$bounds.width;height=$bounds.height;containsPoint=$contains}
   }
   Check ($case.name+': vector point excludes observed native overlay text') $outside @{screenX=$targetX;screenY=$targetY;labels=$rects}
  }
  Point $case.x $case.y
  if($case.ContainsKey('shift')){if([PaxControlsInput]::Key(16,$false) -ne 1){throw 'Shift down failed'};$script:shiftDown=$true;Start-Sleep -Milliseconds 120}
  try{if([PaxControlsInput]::Mouse($(if($case.axis -eq 'x'){0x1000}else{0x800}),$(if($case.axis -eq 'x'){120}else{-120})) -ne 1){throw 'Wheel injection failed'};Start-Sleep -Milliseconds 650}finally{if($script:shiftDown){if([PaxControlsInput]::Key(16,$true) -eq 1){$script:shiftDown=$false}}}
  $after=Snapshot;$report.cases+=@{name=$case.name;before=$before;after=$after};Save
  $counts=@($after.inner_events,$after.outer_events,$after.root_events,$after.overlay_events)
  Check ($case.name+': exact handler chain once') (($counts -join ',') -eq ($case.path -join ',')) @{expected=$case.path;actual=$counts}
  Check ($case.name+': window-space raw event') ($after.raw.Contains(('at ({0:F1}, {1:F1})' -f [double]$case.x,[double]$case.y)) -and $after.raw.Contains($(if($case.axis -eq 'x'){'raw dx=120.000 dy=0.000'}else{'raw dx=0.000 dy=120.000'}))) $after.raw
  if($case.ContainsKey('shift')){Check 'Event-time Shift modifier' ($after.raw.Contains('modifiers=Shift')) $after.raw}
  $inner=[Math]::Abs($after.inner_x)+[Math]::Abs($after.inner_y);$outer=[Math]::Abs($after.outer_x)+[Math]::Abs($after.outer_y)
  $movement=if($case.owner -eq 'inner'){$inner -gt 1 -and $outer -lt 0.05}elseif($case.owner -eq 'outer'){$outer -gt 1 -and $inner -lt 0.05}else{$inner+$outer -lt 0.05}
  if($case.owner -ne 'none' -and !$case.ContainsKey('shift')){
   $otherAxis=if($case.axis -eq 'x'){'y'}else{'x'}
   $movement=$movement -and [Math]::Abs($after[$case.owner+'_'+$case.axis]) -gt 1 -and [Math]::Abs($after[$case.owner+'_'+$otherAxis]) -lt 0.05
  }
  Check ($case.name+': native movement owner') $movement $after
  if($case.ContainsKey('cancel')){
   Check ($case.name+': ancestor cancellation enabled') ($after.prevented -and $after.raw.Contains(' prevented=true]')) $after.raw
  }else{
   Check ($case.name+': cancellation remains disabled') (!$after.prevented -and $after.raw.Contains(' prevented=false]')) $after.raw
  }
  # Two template peers from one viewport must not stand in for both authored scrollers.
  $nativeKinds=@($after.native|ForEach-Object{$_.kind}|Sort-Object)
  $nativeIds=@($after.native|ForEach-Object{$_.id}|Sort-Object -Unique)
  $ownedPeers=@($after.native|Where-Object{$_.id -cmatch '^pax-node-[0-9]+$' -and $_.class -ceq 'ScrollViewer'})
  Check ($case.name+': one owned native peer per authored scroller') ($after.native.Count -eq 2 -and ($nativeKinds -join ',') -ceq 'inner,outer' -and $nativeIds.Count -eq 2 -and $ownedPeers.Count -eq 2) $after.native
  foreach($native in $after.native){Check ($case.name+': native/bound offsets '+$native.kind) ([Math]::Abs($native.x-$after[$native.kind+'_x']) -lt 1 -and [Math]::Abs($native.y-$after[$native.kind+'_y']) -lt 1) $native}
 }
 Screenshot 'final';[void]$process.CloseMainWindow();Check 'Clean native fixture exit' ($process.WaitForExit(15000) -and $process.ExitCode -eq 0) $process.ExitCode;$report.result='passed'
}catch{$report.result='failed';$report.error=$_.ToString();try{Screenshot 'failure'}catch{}}finally{
 try{
  try{if($script:shiftDown){if([PaxControlsInput]::Key(16,$true) -ne 1){throw 'Shift release failed'};$script:shiftDown=$false}}catch{$report.cleanupErrors+=@($_.ToString())}
  try{
   Close-WheelProcess $process
  }catch{$report.cleanupErrors+=@($_.ToString())}
  Read-RetirementEvidence $env:PAX_WINDOWS_HOST_LOG
  if($report.cleanupErrors.Count -or !$report.Contains('close') -or !$report.close.graceful -or $report.close.exitCode -ne 0){
   $report.result='failed';if(!$report.error){$report.error='Fixture did not close normally; see cleanup/close receipt'}
  }
 }finally{
  foreach($entry in $oldEnvironment.GetEnumerator()){[Environment]::SetEnvironmentVariable($entry.Key,$entry.Value,'Process')}
  [void][PaxControlsInput]::SetThreadDpiAwarenessContext($oldDpi)
  if($report.result -eq 'passed'){$report.result='pass'}
  $report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save
 }
}
if($report.result -ne 'pass'){throw $report.error}
