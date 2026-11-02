param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$ImeUiaExecutable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [double]$ExpectedTextScale=1,
    [int]$AttachProcessId=0,
    [string]$AppStderrPath,
    [string]$HostLogPath,
    [string]$EngineLogPath,
    [string]$PreviousResultPath,
    [switch]$KeepAlive,
    [switch]$SnapshotOnly,
    [string]$ExpectedHostSha256=''
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Windows.Forms,System.Drawing
Add-Type @'
using System;using System.Runtime.InteropServices;
public static class PaxTextLayoutWindow {
 [StructLayout(LayoutKind.Sequential)]public struct POINT{public int X,Y;}
 [StructLayout(LayoutKind.Sequential)]public struct RECT{public int Left,Top,Right,Bottom;}
 [DllImport("user32.dll")]public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
 [DllImport("user32.dll")]public static extern uint GetDpiForWindow(IntPtr window);
 [DllImport("user32.dll")]public static extern bool ClientToScreen(IntPtr window,ref POINT point);
 [DllImport("user32.dll")]public static extern bool MoveWindow(IntPtr window,int x,int y,int width,int height,bool repaint);
 [DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr window);
 [DllImport("user32.dll")]public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr window,out RECT rect);
 [DllImport("user32.dll")]public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")]public static extern bool GetCursorPos(out POINT point);
 [DllImport("user32.dll")]public static extern IntPtr WindowFromPoint(POINT point);
 [DllImport("user32.dll")]public static extern IntPtr GetAncestor(IntPtr window,uint flags);
 [DllImport("user32.dll")]public static extern uint GetWindowThreadProcessId(IntPtr window,out uint pid);
 [StructLayout(LayoutKind.Sequential)]public struct MOUSEINPUT{public int dx,dy;public uint mouseData,flags,time;public IntPtr extra;}
 [StructLayout(LayoutKind.Sequential)]public struct INPUT{public uint type;public MOUSEINPUT mouse;}
 [DllImport("user32.dll",SetLastError=true)]static extern uint SendInput(uint count,INPUT[] inputs,int size);
 public static uint Click(){INPUT[] inputs=new INPUT[2];inputs[0].mouse.flags=2;inputs[1].mouse.flags=4;return SendInput(2,inputs,Marshal.SizeOf(typeof(INPUT)));}

}
'@
$Executable=(Resolve-Path -LiteralPath $Executable).Path
$ImeUiaExecutable=(Resolve-Path -LiteralPath $ImeUiaExecutable).Path
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ([Guid]::NewGuid().ToString())
[void](New-Item -ItemType Directory -Force $run)
$previousDpi=[PaxTextLayoutWindow]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$previousLog=$env:PAX_WINDOWS_HOST_LOG
$previousLayoutLog=$env:PAX_TEXT_LAYOUT_LOG
$process=$null;$root=$null;$ownedProcess=$false
$report=[ordered]@{result='incomplete';startedUtc=[DateTime]::UtcNow.ToString('o');executable=$Executable;executableSha256=(Get-FileHash $Executable -Algorithm SHA256).Hash;checkerSha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash;helperSha256=(Get-FileHash $ImeUiaExecutable -Algorithm SHA256).Hash;expectedTextScale=$ExpectedTextScale;attached=($AttachProcessId -ne 0);checks=@();snapshots=@()}
function Check([string]$Name,[bool]$Passed,$Detail){$script:report.checks+=@{name=$Name;passed=$Passed;detail=$Detail}}
function LogText([string]$Path){
 if(-not (Test-Path -LiteralPath $Path)){return ''}
 # The app keeps redirected logs open; read concurrently without disturbing it.
 $stream=[IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
 $reader=New-Object IO.StreamReader($stream)
 try{return $reader.ReadToEnd()}finally{$reader.Dispose()}
}
function Invoke-Button([string]$Name){
 $condition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::NameProperty,$Name)
 $button=$script:root.FindFirst([Windows.Automation.TreeScope]::Descendants,$condition)
 if($null -eq $button){throw "Missing native button: $Name"}
 $button.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
}
function Engine-Snapshot {
 $before=(LogText $EngineLogPath).Length
 Invoke-Button 'Record layout'
 $until=[DateTime]::UtcNow.AddSeconds(5);$matches=@()
 do{
  Start-Sleep -Milliseconds 50
  $text=LogText $EngineLogPath
  if($text.Length -ge $before){$text=$text.Substring($before)}
  $matches=@([regex]::Matches($text,'pax-text-layout: name=(\S+) id=(\d+) width=([^ ]+) height=([^ ]+) x=([^ ]+) y=([^ ]+) measured=(.*?) expanded=(true|false) narrow=(true|false)'))
 }while($matches.Count -lt 10 -and [DateTime]::UtcNow -lt $until)
 if($matches.Count -lt 10){throw 'Engine layout record deadline'}
 $nodes=@{}
 foreach($m in $matches){$nodes[$m.Groups[1].Value]=@{id=[int]$m.Groups[2].Value;width=[double]$m.Groups[3].Value;height=[double]$m.Groups[4].Value;x=[double]$m.Groups[5].Value;y=[double]$m.Groups[6].Value;measured=$m.Groups[7].Value;expanded=($m.Groups[8].Value -eq 'true');narrow=($m.Groups[9].Value -eq 'true')}}
 if($nodes.Count -ne 10){throw 'Expected all ten named engine nodes'}
 return $nodes
}
function Snapshot([string]$Name){
 $hwnd=$process.MainWindowHandle
 [void][PaxTextLayoutWindow]::SetForegroundWindow($hwnd)
 Start-Sleep -Milliseconds 150
 if([PaxTextLayoutWindow]::GetForegroundWindow() -ne $hwnd){
  # Scheduled workers can be denied foreground. Click only a visible title-bar
  # hit belonging to this verified process, never an occluding Settings window.
  [uint32]$owner=0;[void][PaxTextLayoutWindow]::GetWindowThreadProcessId($hwnd,[ref]$owner)
  $bounds=New-Object PaxTextLayoutWindow+RECT;$read=[PaxTextLayoutWindow]::GetWindowRect($hwnd,[ref]$bounds)
  $dpiScale=[PaxTextLayoutWindow]::GetDpiForWindow($hwnd)/96.0
  $hitPoint=New-Object PaxTextLayoutWindow+POINT;$hitPoint.X=$bounds.Left+[int][Math]::Round(100*$dpiScale);$hitPoint.Y=$bounds.Top+[int][Math]::Round(16*$dpiScale)
  $hit=[PaxTextLayoutWindow]::GetAncestor([PaxTextLayoutWindow]::WindowFromPoint($hitPoint),2)
  if(-not $ownedProcess -or $owner -ne $process.Id -or -not $read -or $dpiScale -le 0 -or $hit -ne $hwnd){throw 'Owned visible title-bar foreground precondition failed'}
  $actual=New-Object PaxTextLayoutWindow+POINT
  if(-not [PaxTextLayoutWindow]::SetCursorPos($hitPoint.X,$hitPoint.Y) -or -not [PaxTextLayoutWindow]::GetCursorPos([ref]$actual) -or $actual.X -ne $hitPoint.X -or $actual.Y -ne $hitPoint.Y){throw 'Foreground pointer positioning failed'}
  if([PaxTextLayoutWindow]::Click() -ne 2){throw 'Foreground title-bar click incomplete'}
  Start-Sleep -Milliseconds 200
 }
 if([PaxTextLayoutWindow]::GetForegroundWindow() -ne $hwnd){throw 'Text observations require foreground ownership'}
 Start-Sleep -Milliseconds 500
 $nodes=Engine-Snapshot;Start-Sleep -Milliseconds 250;$settled=Engine-Snapshot
 $stable=$true;foreach($key in $nodes.Keys){foreach($property in @('width','height','x','y')){if([Math]::Abs($nodes[$key][$property]-$settled[$key][$property]) -gt 0.1){$stable=$false}}}
 Check "$Name engine layout settled" $stable $settled
 $nodes=$settled;$dpi=[PaxTextLayoutWindow]::GetDpiForWindow($process.MainWindowHandle);$scale=$dpi/96.0
 $textScale=([Windows.UI.ViewManagement.UISettings,Windows.UI.ViewManagement,ContentType=WindowsRuntime]::new()).TextScaleFactor
 Check "$Name actual text scale" ([Math]::Abs($textScale-$ExpectedTextScale) -lt 0.001) @{actual=$textScale;expected=$ExpectedTextScale;windowDpi=$dpi}
 $point=New-Object PaxTextLayoutWindow+POINT;[void][PaxTextLayoutWindow]::ClientToScreen($process.MainWindowHandle,[ref]$point)
 $snapshot=[ordered]@{name=$Name;observedUtc=[DateTime]::UtcNow.ToString('o');dpi=$dpi;textScaleFactor=$textScale;clientOrigin=@($point.X,$point.Y);engine=$nodes;native=@{}}
 $script:report.snapshots+=@($snapshot)
 $lineHeight=0.0
 foreach($key in @('intrinsic','fixed','paragraph','stack_text','frame_text','repeat_text_0','repeat_text_1')){
  $node=$nodes[$key];$id='pax-node-'+$node.id;$condition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::AutomationIdProperty,$id)
  $element=$root.FindFirst([Windows.Automation.TreeScope]::Descendants,$condition)
  if($null -eq $element){throw "Missing native Text peer $id"}
  $bounds=$element.Current.BoundingRectangle
  $stdout=Join-Path $run "$Name-$key.jsonl";$stderr=Join-Path $run "$Name-$key.stderr.log"
  $probe=Start-Process $ImeUiaExecutable -ArgumentList @([string]$process.Id,$id) -NoNewWindow -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr;$probeHandle=$probe.Handle
  if(-not $probe.WaitForExit(20000)){$probe.Kill();throw "Native UIA helper deadline: $key"};$probe.WaitForExit()
  $rows=@(Get-Content -LiteralPath $stdout|ForEach-Object{$_|ConvertFrom-Json})
  $doc=@($rows|Where-Object{$_.stage -ceq 'document.text'})
  $rect=@($rows|Where-Object{$_.stage -ceq 'document.rectanglesPhysical'})
  $boxes=@();if($rect.Count){$boxes=@($rect[-1].rectangles)}
  $terminalSelection=@($rows|Where-Object{$_.stage -ceq 'document.terminal.selection'})
  $terminalRect=@($rows|Where-Object{$_.stage -ceq 'document.terminal.rectanglesPhysical'})
  $terminalText=@($rows|Where-Object{$_.stage -ceq 'document.terminal.text'})
  $terminalEndpoints=@($rows|Where-Object{$_.stage -ceq 'document.terminal.endpoints'})
  $terminalBoxes=@();if($terminalRect.Count){$terminalBoxes=@($terminalRect[-1].rectangles)}
  $native=@{automationId=$id;name=$element.Current.Name;boundsPhysical=@($bounds.X,$bounds.Y,$bounds.Width,$bounds.Height);offscreen=$element.Current.IsOffscreen;helperExit=$probe.ExitCode;text=if($doc.Count){$doc[-1].text}else{''};rectanglesPhysical=$boxes;terminalRectanglesPhysical=$terminalBoxes}
  $snapshot.native[$key]=$native
  if($key -eq 'intrinsic'){
   Check "$Name single-line reference is available" ($boxes.Count -eq 4 -and $boxes[2] -gt 0 -and $boxes[3] -gt 0) $boxes
   if($boxes.Count -eq 4){$lineHeight=[double]$boxes[3]}
  }
  Check "$Name $key native text range available" ($probe.ExitCode -eq 0 -and $doc.Count -eq 1 -and $doc[0].hresult -ceq '0x00000000' -and $rect.Count -eq 1 -and $rect[0].hresult -ceq '0x00000000') $native
  # UIA describes the intrinsic TextBlock inside the authored box. Only
  # implicit axes must equal engine measurements; constrained width can be wider.
  $aligned=($bounds.Width/$scale -le $node.width+1.1 -and ($key -ne 'intrinsic' -or [Math]::Abs($bounds.Width/$scale-$node.width) -le 1.1) -and ($key -eq 'fixed' -or [Math]::Abs($bounds.Height/$scale-$node.height) -le 1.1))
  Check "$Name $key intrinsic content matches authored constraints" $aligned @{engine=@($node.width,$node.height);native=@(($bounds.Width/$scale),($bounds.Height/$scale))}
  Check "$Name $key engine/native position agrees" ([Math]::Abs(($bounds.X-$point.X)/$scale-$node.x) -le 1.1 -and [Math]::Abs(($bounds.Y-$point.Y)/$scale-$node.y) -le 1.1) @{engine=@($node.x,$node.y);native=@((($bounds.X-$point.X)/$scale),(($bounds.Y-$point.Y)/$scale))}
  if($key -ne 'fixed'){
   Check "$Name $key engine has accepted native measurement" ($node.measured -like 'Some*') $node.measured
   $visible=($node.width -gt 0 -and $node.height -gt 0 -and -not $native.offscreen -and $boxes.Count -gt 0 -and $boxes.Count%4 -eq 0)
   for($i=0;$i+3 -lt $boxes.Count;$i+=4){if($boxes[$i+2] -le 0 -or $boxes[$i+3] -le 0 -or $boxes[$i+1] -lt $bounds.Y-2 -or $boxes[$i+1]+$boxes[$i+3] -gt $bounds.Bottom+2){$visible=$false}}
   # Whole-line UIA rectangles can include trailing whitespace past a constrained width.
   # Vertical fit and the exact final character are the layout guarantees checked here.
   Check "$Name $key measured lines fit vertically" $visible $native
   $documentText=$native.text.TrimEnd([char[]]@([char]13,[char]10))
   $terminalVisible=($documentText.Length -gt 0 -and $terminalText.Count -eq 1 -and $terminalText[0].hresult -ceq '0x00000000' -and $terminalText[0].text -ceq $documentText.Substring($documentText.Length-1) -and $terminalEndpoints.Count -eq 1 -and $terminalEndpoints[0].hresult -ceq '0x00000000' -and $terminalEndpoints[0].startToEnd -lt 0 -and $terminalSelection.Count -eq 1 -and $terminalSelection[0].hresult -ceq '0x00000000' -and $terminalSelection[0].nonWhitespace -and $terminalRect.Count -eq 1 -and $terminalRect[0].hresult -ceq '0x00000000' -and $terminalBoxes.Count -gt 0 -and $terminalBoxes.Count%4 -eq 0)
   for($i=0;$i+3 -lt $terminalBoxes.Count;$i+=4){if($terminalBoxes[$i+2] -le 0 -or $terminalBoxes[$i+3] -le 0 -or $lineHeight -le 0 -or [Math]::Abs($terminalBoxes[$i+3]-$lineHeight) -gt 1.1 -or $terminalBoxes[$i] -lt $bounds.X-2 -or $terminalBoxes[$i+1] -lt $bounds.Y-2 -or $terminalBoxes[$i]+$terminalBoxes[$i+2] -gt $bounds.Right+2 -or $terminalBoxes[$i+1]+$terminalBoxes[$i+3] -gt $bounds.Bottom+2){$terminalVisible=$false}}
   Check "$Name $key terminal text is visible" $terminalVisible $native
  }
 }
 if($textScale -ge 2){
  $fixed=$snapshot.native.fixed;$fixedClips=($fixed.terminalRectanglesPhysical.Count -eq 0)
  for($i=0;$i+3 -lt $fixed.terminalRectanglesPhysical.Count;$i+=4){$b=$fixed.terminalRectanglesPhysical;if($b[$i+3] -lt $lineHeight-1.1 -or $b[$i+1]+$b[$i+3] -gt $fixed.boundsPhysical[1]+$fixed.boundsPhysical[3]+2){$fixedClips=$true}}
  Check "$Name explicit-clip fixed-height negative control exposes clipping" ($lineHeight -gt $nodes.fixed.height*$scale -and $fixedClips) $fixed
 }
 Check "$Name repeated siblings agree with independent IDs" ($nodes.repeat_text_0.id -ne $nodes.repeat_text_1.id -and [Math]::Abs($nodes.repeat_text_0.height-$nodes.repeat_text_1.height) -lt 0.1 -and [Math]::Abs($nodes.repeat_text_0.width-$nodes.repeat_text_1.width) -lt 0.1) @{first=$nodes.repeat_text_0;second=$nodes.repeat_text_1}
 Check "$Name explicit height stays30" ([Math]::Abs($nodes.fixed.height-30) -lt 0.1) $nodes.fixed
 Check "$Name Stacker follows measured text" ([Math]::Abs($nodes.follower.y-($nodes.stack_text.y+$nodes.stack_text.height+12)) -lt 1.1 -and $nodes.stack.height -ge $nodes.stack_text.height+60-1.1) @{stack=$nodes.stack;label=$nodes.stack_text;follower=$nodes.follower}
 Check "$Name Frame includes padding and measured child" ($nodes.frame.height -ge $nodes.frame_text.height+24-1.1 -and [Math]::Abs($nodes.frame_text.y-$nodes.frame.y-12) -lt 1.1) @{frame=$nodes.frame;label=$nodes.frame_text}
 [void][PaxTextLayoutWindow]::SetForegroundWindow($process.MainWindowHandle);Start-Sleep -Milliseconds 100
 Check "$Name screenshot owns foreground" ([PaxTextLayoutWindow]::GetForegroundWindow() -eq $process.MainWindowHandle) $process.MainWindowHandle.ToInt64()
 $window=New-Object PaxTextLayoutWindow+RECT;[void][PaxTextLayoutWindow]::GetWindowRect($process.MainWindowHandle,[ref]$window)
 $bitmap=New-Object Drawing.Bitmap(($window.Right-$window.Left),($window.Bottom-$window.Top));$graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{$graphics.CopyFromScreen($window.Left,$window.Top,0,0,$bitmap.Size);$bitmap.Save((Join-Path $run "$Name.png"))}finally{$graphics.Dispose();$bitmap.Dispose()}
 $snapshot|ConvertTo-Json -Depth 12|Set-Content -Encoding UTF8 (Join-Path $run "$Name.json")
 return $snapshot
}
try{
 if([Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0){throw 'Interactive desktop required'}
 if($AttachProcessId){
  if(-not $AppStderrPath -or -not $HostLogPath -or -not $EngineLogPath){throw 'Attach requires original stderr, host and engine log paths'}
  $process=Get-Process -Id $AttachProcessId
 }else{
  if(Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($Executable)) -ErrorAction SilentlyContinue){throw 'Competing fixture process'}
  $AppStderrPath=Join-Path $run 'app.stderr.log';$HostLogPath=Join-Path $run 'host.log';$env:PAX_WINDOWS_HOST_LOG=$HostLogPath
  $EngineLogPath=Join-Path $run 'engine.log';$env:PAX_TEXT_LAYOUT_LOG=$EngineLogPath
  $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $run 'app.stdout.log') -RedirectStandardError $AppStderrPath
  $ownedProcess=$true
 }
 $appHandle=$process.Handle;$report.processId=$process.Id;$report.appStderrPath=$AppStderrPath;$report.hostLogPath=$HostLogPath;$report.engineLogPath=$EngineLogPath
 $until=[DateTime]::UtcNow.AddSeconds(60)
 do{Start-Sleep -Milliseconds 200;$process.Refresh();if($process.HasExited){throw 'Fixture exited before first frame'}}while(($process.MainWindowHandle -eq [IntPtr]::Zero -or -not (LogText $HostLogPath).Contains('first_frame_published source=pax-runtime')) -and [DateTime]::UtcNow -lt $until)
 if($process.Path -ine $Executable -or -not (LogText $HostLogPath).Contains('first_frame_published source=pax-runtime')){throw 'Owned first-frame precondition failed'}
 $ownedProcess=$true;$report.processStartUtc=$process.StartTime.ToUniversalTime().ToString('o')
 $loadedHost=@($process.Modules|Where-Object{$_.ModuleName -ieq 'PaxHost.dll'});$report.loadedHostPath=if($loadedHost.Count -eq 1){$loadedHost[0].FileName}else{''};$report.hostSha256=if($loadedHost.Count -eq 1){(Get-FileHash $loadedHost[0].FileName).Hash}else{''}
 Check 'Expected native DLL loaded' ($loadedHost.Count -eq 1 -and (!$ExpectedHostSha256 -or $report.hostSha256 -ieq $ExpectedHostSha256)) @{path=$report.loadedHostPath;sha256=$report.hostSha256;expected=$ExpectedHostSha256}

 $screen=[Windows.Forms.Screen]::FromHandle($process.MainWindowHandle)
 $report.desktopBounds=@($screen.Bounds.X,$screen.Bounds.Y,$screen.Bounds.Width,$screen.Bounds.Height)
 $report.workingArea=@($screen.WorkingArea.X,$screen.WorkingArea.Y,$screen.WorkingArea.Width,$screen.WorkingArea.Height)
 if(-not $AttachProcessId){
  $work=$screen.WorkingArea
  if($work.Width -lt 1040 -or $work.Height -lt 1020){throw '1000x1000 test window requires a larger desktop working area'}
  if(-not [PaxTextLayoutWindow]::MoveWindow($process.MainWindowHandle,$work.X+20,$work.Y+10,1000,1000,$true)){throw 'Initial window placement failed'}
 }
 $root=[Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
 # Capture live scale changes before a content/width edit can trigger remeasurement.
 $initial=Snapshot 'initial'
 if($PreviousResultPath){
  $previous=Get-Content -LiteralPath $PreviousResultPath -Raw|ConvertFrom-Json
  $prior=@($previous.snapshots)[-1]
  Check 'Live comparison has successful unchanged process identity' ($previous.result -like 'passed*' -and $previous.processLeftRunning -and $previous.processStartUtc -ceq $report.processStartUtc -and $previous.executableSha256 -ceq $report.executableSha256 -and $previous.helperSha256 -ceq $report.helperSha256 -and $prior.dpi -eq $initial.dpi) @{previousStart=$previous.processStartUtc;currentStart=$report.processStartUtc;previousDpi=$prior.dpi;currentDpi=$initial.dpi}
  Check 'Live comparison uses same process and content state' ($AttachProcessId -eq $previous.processId -and $initial.engine.paragraph.expanded -eq $prior.engine.paragraph.expanded -and $initial.engine.paragraph.narrow -eq $prior.engine.paragraph.narrow) @{previousProcess=$previous.processId;currentProcess=$process.Id;previousSnapshot=$prior.name}
  $direction=[Math]::Sign($initial.textScaleFactor-$prior.textScaleFactor)
  Check 'Live comparison changes actual system text scale' ($direction -ne 0) @{before=$prior.textScaleFactor;after=$initial.textScaleFactor}
  foreach($key in @('intrinsic','paragraph','stack_text','frame_text','repeat_text_0','repeat_text_1')){
   $heightDelta=$initial.engine[$key].height-$prior.engine.$key.height
   Check "Live scale remeasures $key height before any content edit" ($heightDelta*$direction -gt 0) @{before=$prior.engine.$key;after=$initial.engine[$key]}
  }
  Check 'Live scale remeasures unconstrained width' (($initial.engine.intrinsic.width-$prior.engine.intrinsic.width)*$direction -gt 0) @{before=$prior.engine.intrinsic;after=$initial.engine.intrinsic}
 }

 if(-not $SnapshotOnly){
  if($initial.engine.paragraph.expanded){Invoke-Button 'Toggle content'}
  if($initial.engine.paragraph.narrow){Invoke-Button 'Toggle width'}
  $short=Snapshot 'short-wide'
  Invoke-Button 'Toggle content';$long=Snapshot 'long-wide'
  Check 'Long content increases measured paragraph height' ($long.engine.paragraph.height -gt $short.engine.paragraph.height) @{short=$short.engine.paragraph;long=$long.engine.paragraph}
  foreach($key in @('repeat_text_0','repeat_text_1')){Check "Content update preserves $key measurement" ($long.engine[$key].height -ge $short.engine[$key].height -and $long.native[$key].text -cne $short.native[$key].text) @{short=$short.engine[$key];long=$long.engine[$key]}}
  Invoke-Button 'Toggle width';$narrow=Snapshot 'long-narrow'
  Check 'Narrow width increases measured paragraph height' ($narrow.engine.paragraph.height -gt $long.engine.paragraph.height -and [Math]::Abs($narrow.engine.paragraph.width-180) -lt 0.1) @{wide=$long.engine.paragraph;narrow=$narrow.engine.paragraph}
 }
 if(-not $KeepAlive){
  if(-not $process.CloseMainWindow() -or -not $process.WaitForExit(10000)){throw 'Normal native close deadline'};$process.WaitForExit();$report.exitCode=$process.ExitCode
  $closed=[regex]::Match((LogText $HostLogPath),'closed_gpu=(\{[^\r\n]+\})');$gpu=if($closed.Success){$closed.Groups[1].Value|ConvertFrom-Json}else{$null}
  Check 'Normal close retires all owned exports' ($process.ExitCode -eq 0 -and $null -ne $gpu -and $gpu.pendingFrames -eq 0 -and $gpu.allocatedImages -eq 0 -and $gpu.allocatedBytes -eq 0 -and $gpu.retired -eq $gpu.published -and (LogText $HostLogPath).Contains('closed exports_drained=true')) $gpu
 }
 $failed=@($report.checks|Where-Object{-not $_.passed});$report.result=if($failed.Count){'failed native text layout assertions'}else{'passed bounded native text layout assertions'}
}catch{$report.result='failed qualification precondition or observation';$report.error=$_.Exception.Message;$report.errorLocation=$_.InvocationInfo.PositionMessage;$report.scriptStackTrace=$_.ScriptStackTrace}
finally{
 if($ownedProcess -and $process -and -not $process.HasExited -and -not $KeepAlive){[void]$process.CloseMainWindow();[void]$process.WaitForExit(10000)}
 if($process){$report.processLeftRunning=(-not $process.HasExited)}
 $env:PAX_WINDOWS_HOST_LOG=$previousLog
 $env:PAX_TEXT_LAYOUT_LOG=$previousLayoutLog
 if($previousDpi -ne [IntPtr]::Zero){[void][PaxTextLayoutWindow]::SetThreadDpiAwarenessContext($previousDpi)}
 $report.finishedUtc=[DateTime]::UtcNow.ToString('o');$result=Join-Path $run 'result.json';$report|ConvertTo-Json -Depth 14|Set-Content -Encoding UTF8 $result;Write-Output $result
}
if($report.result -notlike 'passed*'){exit 1}
