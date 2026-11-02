param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PaxKeyboardInput {
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
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ([Guid]::NewGuid().ToString())
[void](New-Item -ItemType Directory -Force -Path $run)
$log=Join-Path $run 'native-host.log';$previousLog=$env:PAX_WINDOWS_HOST_LOG;$env:PAX_WINDOWS_HOST_LOG=$log
$previousInputDiagnostics=$env:PAX_NATIVE_INPUT_DIAGNOSTICS;$env:PAX_NATIVE_INPUT_DIAGNOSTICS='1'
$previousDpi=[PaxKeyboardInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$process=$null;$held=New-Object 'System.Collections.Generic.HashSet[int]'
$closeProbe=$false;$closeExpectedUp=0
$report=[ordered]@{result='running';startedAtUtc=[DateTime]::UtcNow.ToString('o');executable=$Executable;sha256=(Get-FileHash $Executable -Algorithm SHA256).Hash;checks=@();ime='not qualified; native editing isolation only';accessibility='focus ownership and Tab only; screen-reader navigation not qualified'}
function Save([string]$Stage) {
    $report.checkpoint=$Stage;$report.checkpointAtUtc=[DateTime]::UtcNow.ToString('o')
    $report|ConvertTo-Json -Depth 10|Set-Content -LiteralPath (Join-Path $run 'result.tmp.json') -Encoding UTF8
    Move-Item -LiteralPath (Join-Path $run 'result.tmp.json') -Destination (Join-Path $run 'result.json') -Force
}
function Check([string]$Name,[bool]$Passed,$Detail) {
    $report.checks+=@{name=$Name;passed=$Passed;detail=$Detail};Save $Name
    if(-not $Passed){throw "$Name failed: $($Detail|ConvertTo-Json -Compress -Depth 5)"}
}
function Host-Log {if(Test-Path -LiteralPath $log){return [IO.File]::ReadAllText($log)};return ''}
function Names {
    Save 'Read native text labels'
    $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Text)
    $all=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
    $names=@();for($i=0;$i -lt $all.Count;$i++){$names+=,$all.Item($i).Current.Name};return $names
}
function Wait-Label([string]$Name) {
    $until=[DateTime]::UtcNow.AddSeconds(10)
    do{if(@(Names) -ccontains $Name){return};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $until)
    throw "Missing label: $Name"
}
function Stats {
    $labels=@(Names|Where-Object{$_ -match '^Keys: down=(\d+) up=(\d+) press=(\d+) repeats=(\d+)$'})
    if($labels.Count -ne 1){throw "Expected one keyboard counter, found $($labels.Count)"}
    [void]($labels[0] -match '^Keys: down=(\d+) up=(\d+) press=(\d+) repeats=(\d+)$')
    return @{down=[int]$Matches[1];up=[int]$Matches[2];press=[int]$Matches[3];repeats=[int]$Matches[4];label=$labels[0]}
}
function Wait-Stats([scriptblock]$Predicate) {
    $until=[DateTime]::UtcNow.AddSeconds(10)
    do{$current=Stats;if(& $Predicate $current){return $current};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $until)
    throw "Keyboard counters did not reach expected state: $($current.label)"
}
function Key([int]$Code,[bool]$Up=$false) {
    if([PaxKeyboardInput]::GetForegroundWindow() -ne $process.MainWindowHandle){throw 'Refusing key injection: fixture lost foreground'}
    if([PaxKeyboardInput]::Key([uint16]$Code,$Up) -ne 1){throw 'SendInput key delivery failed'}
    if($Up){[void]$held.Remove($Code)}else{[void]$held.Add($Code)}
}
function Tap([int]$Code) {Key $Code;Key $Code $true;Start-Sleep -Milliseconds 100}
function Point([int]$X,[int]$Y,[bool]$Click=$false) {
    $position=New-Object PaxKeyboardInput+POINT;$position.X=$X;$position.Y=$Y
    $owner=[PaxKeyboardInput]::GetAncestor([PaxKeyboardInput]::WindowFromPoint($position),2)
    Check 'Pointer target belongs to fixture' ($owner -eq $process.MainWindowHandle) @($X,$Y,$owner.ToInt64())
    $moved=[PaxKeyboardInput]::SetCursorPos($X,$Y);$actual=New-Object PaxKeyboardInput+POINT
    $read=[PaxKeyboardInput]::GetCursorPos([ref]$actual)
    Check 'Pointer reaches observed target' ($moved -and $read -and $actual.X -eq $X -and $actual.Y -eq $Y) @($actual.X,$actual.Y)
    if($Click){Check 'Pointer click injected completely' ([PaxKeyboardInput]::Click() -eq 2) @($X,$Y)}
    Start-Sleep -Milliseconds 150
}
function Scroll-State {
    $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty,'ScrollViewer')
    $all=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
    if($all.Count -ne 2){throw "Expected two native scrollers, found $($all.Count)"}
    $positions=@()
    for($i=0;$i -lt $all.Count;$i++){
        $item=$all.Item($i);$pattern=$item.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
        $positions+=,[ordered]@{id=$item.Current.AutomationId;x=$pattern.Current.HorizontalScrollPercent;y=$pattern.Current.VerticalScrollPercent}
    }
    return ($positions|ConvertTo-Json -Compress)
}
function Canvas-Click {
    $origin=New-Object PaxKeyboardInput+POINT
    if(-not [PaxKeyboardInput]::ClientToScreen($process.MainWindowHandle,[ref]$origin)){throw 'Client origin unavailable'}
    $scale=[PaxKeyboardInput]::GetDpiForWindow($process.MainWindowHandle)/96.0
    $bounds=New-Object PaxKeyboardInput+RECT
    if(-not [PaxKeyboardInput]::GetClientRect($process.MainWindowHandle,[ref]$bounds) -or $scale -le 0 -or $bounds.Right/$scale -lt 460 -or $bounds.Bottom/$scale -lt 590){throw 'Vector target is outside the visible scroller at this window size/DPI'}
    # Authored vector-only teal target: outer-scroller (24,304) + (380,220),
    # sampled 20 DIP inside its left edge and 22 DIP inside its top edge.
    $scrollBefore=Scroll-State
    $countBefore=@(Names|Where-Object{$_ -match '^Count: \d+$'})
    if($countBefore.Count -ne 1){throw 'Native button counter unavailable'}
    Point ([int][Math]::Round($origin.X+424*$scale)) ([int][Math]::Round($origin.Y+546*$scale)) $true
    Start-Sleep -Milliseconds 250
    $focused=[System.Windows.Automation.AutomationElement]::FocusedElement
    Check 'Vector click gives the actual Pax canvas keyboard focus' ($focused.Current.ProcessId -eq $process.Id -and $focused.Current.AutomationId -ceq 'pax-canvas' -and $focused.Current.HasKeyboardFocus) @{name=$focused.Current.Name;id=$focused.Current.AutomationId;class=$focused.Current.ClassName;keyboard=$focused.Current.HasKeyboardFocus}
    $scrollAfter=Scroll-State
    Check 'Vector click preserves both native scroller positions' ($scrollAfter -ceq $scrollBefore) @{before=$scrollBefore;after=$scrollAfter}
    $countAfter=@(Names|Where-Object{$_ -match '^Count: \d+$'})
    Check 'Vector click does not activate a native button' ($countAfter.Count -eq 1 -and $countAfter[0] -ceq $countBefore[0]) $countAfter
}
try {
    Save 'start'
    Check 'Interactive desktop' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
    Check 'Physical DPI coordinate mode' ($previousDpi -ne [IntPtr]::Zero) $previousDpi.ToInt64()
    $existing=@(Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($Executable)) -ErrorAction SilentlyContinue)
    Check 'No competing fixture' ($existing.Count -eq 0) @($existing|ForEach-Object{$_.Id})
    $process=Start-Process -FilePath $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $run 'app.stdout.log') -RedirectStandardError (Join-Path $run 'app.stderr.log')
    $retainedHandle=$process.Handle;$report.process=$process.Id
    $until=[DateTime]::UtcNow.AddSeconds(60)
    do{Start-Sleep -Milliseconds 200;$process.Refresh();if($process.HasExited){throw 'Fixture exited during launch'}}while(($process.MainWindowHandle -eq [IntPtr]::Zero -or -not (Host-Log).Contains('first_frame_published source=pax-runtime')) -and [DateTime]::UtcNow -lt $until)
    Check 'Actual runtime first frame' ((Host-Log).Contains('first_frame_published source=pax-runtime')) $process.MainWindowHandle.ToInt64()
    $work=[Windows.Forms.Screen]::FromHandle($process.MainWindowHandle).WorkingArea
    Check 'Visible desktop fits keyboard fixture' ($work.Width -ge 980 -and $work.Height -ge 760) $work.ToString()
    Check 'Place fixture inside desktop' ([PaxKeyboardInput]::MoveWindow($process.MainWindowHandle,$work.Left+20,$work.Top+20,[Math]::Min(1000,$work.Width-40),[Math]::Min(840,$work.Height-40),$true)) $work.ToString()
    [void][PaxKeyboardInput]::SetForegroundWindow($process.MainWindowHandle);Start-Sleep -Milliseconds 200
    if([PaxKeyboardInput]::GetForegroundWindow() -ne $process.MainWindowHandle){
        $bounds=New-Object PaxKeyboardInput+RECT
        if(-not [PaxKeyboardInput]::GetWindowRect($process.MainWindowHandle,[ref]$bounds)){throw 'Window bounds unavailable'}
        Point ($bounds.Left+100) ($bounds.Top+16) $true
    }
    Check 'Fixture owns foreground' ([PaxKeyboardInput]::GetForegroundWindow() -eq $process.MainWindowHandle) $process.MainWindowHandle.ToInt64()
    $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    Wait-Label 'Keys: down=0 up=0 press=0 repeats=0'
    $initialFocus=[System.Windows.Automation.AutomationElement]::FocusedElement
    $initialFocusPassed=$initialFocus.Current.ProcessId -eq $process.Id -and $initialFocus.Current.AutomationId -ceq 'pax-canvas' -and $initialFocus.Current.HasKeyboardFocus
    $report.initialFocus=@{name=$initialFocus.Current.Name;id=$initialFocus.Current.AutomationId;class=$initialFocus.Current.ClassName;keyboard=$initialFocus.Current.HasKeyboardFocus;passed=$initialFocusPassed}
    Save 'Recorded initial focus before any canvas click'
    $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Edit)
    $found=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
    $editors=@();for($i=0;$i -lt $found.Count;$i++){$editors+=,$found.Item($i)}
    $editor=@($editors|Sort-Object {$_.Current.BoundingRectangle.Top})[0]
    Check 'Real native editor available' ($editors.Count -eq 2 -and -not $editor.Current.IsOffscreen) $editor.Current.AutomationId
    Canvas-Click
    $cursor=New-Object PaxKeyboardInput+CURSORINFO;$cursor.Size=[Runtime.InteropServices.Marshal]::SizeOf($cursor)
    $read=[PaxKeyboardInput]::GetCursorInfo([ref]$cursor)
    $handCursorPassed=$read -and $cursor.Cursor -eq [PaxKeyboardInput]::LoadCursor([IntPtr]::Zero,[IntPtr]::new(32649))
    $report.cursor=@{read=$read;handle=$cursor.Cursor.ToInt64();matchesSystemHand=$handCursorPassed}
    Tap 65
    Wait-Label 'KeyDown: key=a mods= repeat=false';Wait-Label 'KeyUp: key=a mods= repeat=false';Wait-Label 'KeyPress: key=a mods= repeat=false'
    $state=Wait-Stats {param($s)$s.down -eq 1 -and $s.up -eq 1 -and $s.press -eq 1}
    Check 'Canvas printable down/up/press' $true $state
    Tap 81
    Wait-Label 'KeyDown: key=q mods= repeat=false';Wait-Label 'KeyUp: key=q mods= repeat=false'
    $state=Wait-Stats {param($s)$s.down -eq 2 -and $s.up -eq 2 -and $s.press -eq 1}
    Check 'Cancelled printable down suppresses KeyPress' $true $state
    Key 17;Key 65;Key 65 $true
    Wait-Label 'KeyDown: key=a mods=Control repeat=false';Wait-Label 'KeyUp: key=a mods=Control repeat=false'
    Key 17 $true
    $state=Wait-Stats {param($s)$s.down -eq 4 -and $s.up -eq 4 -and $s.press -eq 1}
    Check 'Control shortcut preserves key and suppresses KeyPress' $true $state
    Key 65;[void](Wait-Stats {param($s)$s.down -eq 5 -and $s.press -eq 2})
    Key 65;[void](Wait-Stats {param($s)$s.down -eq 6 -and $s.press -eq 3 -and $s.repeats -eq 1})
    Key 65;[void](Wait-Stats {param($s)$s.down -eq 7 -and $s.press -eq 4 -and $s.repeats -eq 2})
    Key 65 $true
    Wait-Label 'KeyDown: key=a mods= repeat=true'
    $state=Wait-Stats {param($s)$s.down -eq 7 -and $s.up -eq 5 -and $s.press -eq 4 -and $s.repeats -eq 2}
    Check 'Repeated deliveries retain one held key' $true $state
    Key 90
    Wait-Label 'KeyDown: key=z mods= repeat=false'
    $editorBounds=$editor.Current.BoundingRectangle
    Point ([int]($editorBounds.Left+$editorBounds.Width/2)) ([int]($editorBounds.Top+$editorBounds.Height/2)) $true
    $focused=[System.Windows.Automation.AutomationElement]::FocusedElement
    Check 'Native editor click retains actual native keyboard focus' ($focused.Current.ProcessId -eq $process.Id -and $focused.Current.AutomationId -ceq $editor.Current.AutomationId -and $focused.Current.HasKeyboardFocus) @{name=$focused.Current.Name;id=$focused.Current.AutomationId;class=$focused.Current.ClassName;keyboard=$focused.Current.HasKeyboardFocus}
    Wait-Label 'KeyUp: key=z mods= repeat=false'
    $released=Stats;Key 90 $true;Start-Sleep -Milliseconds 200
    $after=Stats
    Check 'Native focus releases held canvas key exactly once' ($released.up -eq 6 -and $after.label -ceq $released.label) $after
    [Windows.Forms.SendKeys]::SendWait('^a');[Windows.Forms.SendKeys]::SendWait('native-only')
    Wait-Label 'Rust received: native-only'
    $after=Stats
    Check 'Native editing stays out of global canvas handlers' ($after.label -ceq $released.label) $after
    Canvas-Click;Tap 66
    Wait-Label 'KeyPress: key=b mods= repeat=false'
    $after=Wait-Stats {param($s)$s.down -eq 9 -and $s.up -eq 7 -and $s.press -eq 6}
    Check 'Vector click restores keyboard focus after editing' $true $after
    Key 17;Canvas-Click;Key 17 $true
    Wait-Label 'Pointer: mods=Control'
    Check 'Pointer modifiers reach Rust' $true 'Pointer: mods=Control'
    Check 'Standalone canvas initially owns keyboard focus' $initialFocusPassed $report.initialFocus
    Check 'Vector hover selects actual system hand cursor' $handCursorPassed $report.cursor
    foreach($named in @(@{code=13;name='Enter';press=1},@{code=37;name='ArrowLeft';press=0},@{code=27;name='Escape';press=0})){
        $before=Stats;Tap $named.code
        Wait-Label "KeyDown: key=$($named.name) mods= repeat=false"
        Wait-Label "KeyUp: key=$($named.name) mods= repeat=false"
        $after=Wait-Stats {param($s)$s.down -eq $before.down+1 -and $s.up -eq $before.up+1 -and $s.press -eq $before.press+$named.press}
        if($named.press){Wait-Label "KeyPress: key=$($named.name) mods= repeat=false"}
        Check "$($named.name) preserves named payload and KeyPress policy" $true $after
    }
    $before=Stats;Key 122
    Wait-Label 'KeyDown: key=F11 mods= repeat=false'
    [void](Wait-Stats {param($s)$s.down -eq $before.down+1 -and $s.up -eq $before.up -and $s.press -eq $before.press})
    # This window belongs to this diagnostic process, not the fixture. It gives
    # us a real OS activation change without touching unrelated user windows.
    $other=New-Object Windows.Forms.Form
    try {
        $other.Text='Pax keyboard deactivation probe';$other.StartPosition='Manual'
        $other.Location=New-Object Drawing.Point(($work.Left+40),($work.Top+40))
        $other.Size=New-Object Drawing.Size(320,160);$other.ShowInTaskbar=$false
        $other.Show();$other.Activate();[Windows.Forms.Application]::DoEvents()
        [void][PaxKeyboardInput]::SetForegroundWindow($other.Handle)
        Start-Sleep -Milliseconds 150;[Windows.Forms.Application]::DoEvents()
        Check 'Owned other-process window actually deactivates fixture' ([PaxKeyboardInput]::GetForegroundWindow() -eq $other.Handle) @{foreground=([PaxKeyboardInput]::GetForegroundWindow()).ToInt64();probe=$other.Handle.ToInt64();fixture=$process.MainWindowHandle.ToInt64()}
        Wait-Label 'KeyUp: key=F11 mods= repeat=false'
        $deactivated=Wait-Stats {param($s)$s.down -eq $before.down+1 -and $s.up -eq $before.up+1 -and $s.press -eq $before.press}
        Check 'Window deactivation releases the held canvas key' $true $deactivated
    } finally {
        $other.Close();$other.Dispose()
    }
    [void][PaxKeyboardInput]::SetForegroundWindow($process.MainWindowHandle);Start-Sleep -Milliseconds 150
    Check 'Fixture reactivated after owned probe closes' ([PaxKeyboardInput]::GetForegroundWindow() -eq $process.MainWindowHandle) $process.MainWindowHandle.ToInt64()
    Key 122 $true;Start-Sleep -Milliseconds 200
    $after=Stats
    Check 'Physical release after reactivation does not duplicate KeyUp' ($after.label -ceq $deactivated.label) $after
    Canvas-Click;Tap 9
    $focused=[System.Windows.Automation.AutomationElement]::FocusedElement
    Check 'Tab moves from canvas to a real native control' ($focused.Current.ProcessId -eq $process.Id -and $focused.Current.AutomationId -match '^pax-node-\d+$') @{name=$focused.Current.Name;id=$focused.Current.AutomationId;class=$focused.Current.ClassName}
    $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty,'pax-canvas')
    $canvas=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
    if($null -eq $canvas){throw 'Actual canvas automation peer unavailable for held-key close'}
    $canvas.SetFocus()
    $focused=[System.Windows.Automation.AutomationElement]::FocusedElement
    Check 'Canvas owns final held-key close' ($focused.Current.ProcessId -eq $process.Id -and $focused.Current.AutomationId -ceq 'pax-canvas' -and $focused.Current.HasKeyboardFocus) $focused.Current.AutomationId
    Key 123
    Wait-Label 'KeyDown: key=F12 mods= repeat=false'
    $closeExpectedUp=(Stats).up+1;$closeProbe=$true
    Check 'No terminal host failure' (-not ((Host-Log) -match '(?m)^\d+ error ')) (Host-Log)
    $report.result='interactive checks passed; close pending'
} catch {
    $report.result='failed';$report.error=$_.Exception.Message
    try{
        $report.visibleLabels=@(Names)
        $focused=[Windows.Automation.AutomationElement]::FocusedElement
        $report.focus=@{name=$focused.Current.Name;class=$focused.Current.ClassName;id=$focused.Current.AutomationId;pid=$focused.Current.ProcessId;keyboard=$focused.Current.HasKeyboardFocus}
    }catch{$report.diagnosticError=$_.Exception.Message}
} finally {
    try {
        if($null -ne $process){
            # Keep the injected key physically held until the owned app exits:
            # only the host's close path can deliver this final Rust KeyUp.
            if(-not $process.HasExited){[void]$process.CloseMainWindow()}
            if(-not $process.WaitForExit(25000)){$report.result='failed';$report.closeError='25-second close deadline exceeded';$process.Kill();[void]$process.WaitForExit(5000)}
            $report.exitCode=$process.ExitCode;$report.hostLog=Host-Log
            $clean=$process.ExitCode -eq 0 -and $report.hostLog.Contains('closed exports_drained=true')
            $report.checks+=@{name='Clean process and export retirement';passed=$clean;detail=$process.ExitCode}
            if($closeProbe){
                $stderr=[IO.File]::ReadAllText((Join-Path $run 'app.stderr.log'))
                $releaseMatches=[regex]::Matches($stderr,'(?m)^pax-key-up: key="F12" modifiers="" count=(\d+)\r?$')
                $releasePassed=$releaseMatches.Count -eq 1 -and [int]$releaseMatches[0].Groups[1].Value -eq $closeExpectedUp
                $report.checks+=@{name='Held-key close reaches the actual Rust KeyUp handler exactly once';passed=$releasePassed;detail=@{matches=$releaseMatches.Count;expectedCount=$closeExpectedUp}}
                if(-not $releasePassed){$report.result='failed'}
            }
            if(-not $clean){$report.result='failed'}elseif($report.result -eq 'interactive checks passed; close pending'){$report.result='passed scripted canvas keyboard and native editing isolation checks'}
            $process.Dispose()
        }
    } finally {
        foreach($code in @($held)){[void][PaxKeyboardInput]::Key([uint16]$code,$true)}
        $env:PAX_WINDOWS_HOST_LOG=$previousLog
        $env:PAX_NATIVE_INPUT_DIAGNOSTICS=$previousInputDiagnostics
        if($previousDpi -ne [IntPtr]::Zero){[void][PaxKeyboardInput]::SetThreadDpiAwarenessContext($previousDpi)}
        Save 'finished'
    }
}
Write-Output (Join-Path $run 'result.json')
if($report.result -notlike 'passed*'){exit 1}
