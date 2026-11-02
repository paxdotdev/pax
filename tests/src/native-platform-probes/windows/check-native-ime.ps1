param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$ImeUiaExecutable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [switch]$CheckGeometry
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PaxImeInput {
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int X,Y; public uint Data,Flags,Time; public IntPtr Extra; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort Key,Scan; public uint Flags,Time; public IntPtr Extra; }
    [StructLayout(LayoutKind.Explicit)] public struct UNION { [FieldOffset(0)] public MOUSEINPUT Mouse; [FieldOffset(0)] public KEYBDINPUT Keyboard; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint Type; public UNION Data; }
    [DllImport("user32.dll",SetLastError=true)] static extern uint SendInput(uint count, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("kernel32.dll")] public static extern IntPtr GetConsoleWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window,out uint pid);
    [DllImport("user32.dll")] public static extern IntPtr GetKeyboardLayout(uint thread);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr window,int x,int y,int width,int height,bool repaint);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr window,uint flags);
    public static uint Keys(ushort[] keys,bool[] ups) {
        INPUT[] values=new INPUT[keys.Length];
        for(int i=0;i<keys.Length;i++) { values[i].Type=1; values[i].Data.Keyboard.Key=keys[i]; values[i].Data.Keyboard.Flags=ups[i]?2u:0u; }
        return SendInput((uint)values.Length,values,Marshal.SizeOf(typeof(INPUT)));
    }
    public static uint Click() {
        INPUT down=new INPUT(),up=new INPUT(); down.Data.Mouse.Flags=2; up.Data.Mouse.Flags=4;
        return SendInput(2,new INPUT[]{down,up},Marshal.SizeOf(typeof(INPUT)));
    }
}
'@
$Executable=(Resolve-Path -LiteralPath $Executable).Path
$ImeUiaExecutable=(Resolve-Path -LiteralPath $ImeUiaExecutable).Path
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ([Guid]::NewGuid().ToString())
[void](New-Item -ItemType Directory -Force -Path $run)
$log=Join-Path $run 'native-host.log';$previousLog=$env:PAX_WINDOWS_HOST_LOG;$env:PAX_WINDOWS_HOST_LOG=$log
$previousDpi=[PaxImeInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$process=$null;$root=$null;$editor=$null;$originalLayout=[IntPtr]::Zero;$switchAttempted=$false
$held=New-Object 'System.Collections.Generic.HashSet[int]'
$report=[ordered]@{result='running';startedUtc=[DateTime]::UtcNow.ToString('o');executable=$Executable;sha256=(Get-FileHash $Executable -Algorithm SHA256).Hash;helper=$ImeUiaExecutable;helperSha256=(Get-FileHash $ImeUiaExecutable -Algorithm SHA256).Hash;checks=@();scope='Microsoft Japanese IME, romaji, native TextBox preedit/conversion/candidate/commit/cancel/selection, canvas isolation and close';exclusions=@('Other IMEs and languages','Candidate anchoring after transforms or scrolling','IME reconversion','Screen-reader interaction','IME mode preference restoration')}
if($CheckGeometry){$report.exclusions=@($report.exclusions|Where-Object{$_ -cne 'Candidate anchoring after transforms or scrolling'});$report.scope+='; window translation, 3-degree rotation and nested-scroll candidate anchoring';$report.metrics=@()}
function Save([string]$Stage) {
    $report.stage=$Stage;$report.checkpointUtc=[DateTime]::UtcNow.ToString('o')
    $report|ConvertTo-Json -Depth 14|Set-Content -LiteralPath (Join-Path $run 'result.tmp.json') -Encoding UTF8
    Move-Item -LiteralPath (Join-Path $run 'result.tmp.json') -Destination (Join-Path $run 'result.json') -Force
}
function Check([string]$Name,[bool]$Passed,$Detail) {
    $report.checks+=@{name=$Name;passed=$Passed;detail=$Detail};Save $Name
    if(-not $Passed){throw "$Name failed: $($Detail|ConvertTo-Json -Compress -Depth 5)"}
}
function Host-Log {if(Test-Path -LiteralPath $log){return [IO.File]::ReadAllText($log)};return ''}
function Window-Layout([IntPtr]$Window,[int]$ExpectedPid) {
    [uint32]$owner=0;$thread=[PaxImeInput]::GetWindowThreadProcessId($Window,[ref]$owner)
    if(-not $thread -or $owner -ne $ExpectedPid){throw 'Input-layout target no longer belongs to the expected process'}
    $layout=[PaxImeInput]::GetKeyboardLayout($thread)
    if($layout -eq [IntPtr]::Zero){throw 'Input layout unavailable'}
    return $layout
}
function Assert-Editor {
    $process.Refresh()
    if($process.HasExited -or $process.Path -ine $Executable -or [PaxImeInput]::GetForegroundWindow() -ne $process.MainWindowHandle){throw 'Refusing input: owned fixture lost foreground or process identity'}
    $focus=[Windows.Automation.AutomationElement]::FocusedElement
    if($focus.Current.ProcessId -ne $process.Id -or $focus.Current.AutomationId -cne $editor.Current.AutomationId -or -not $focus.Current.HasKeyboardFocus){throw 'Refusing input: exact native editor lost focus'}
}
function Send-Batch([uint16[]]$Keys,[bool[]]$Up) {
    if($Keys.Length -ne $Up.Length){throw 'Invalid input batch'}
    $sent=[PaxImeInput]::Keys($Keys,$Up)
    # Track only events accepted from this diagnostic, including a partial batch.
    for($i=0;$i -lt $sent;$i++){if($Up[$i]){[void]$held.Remove($Keys[$i])}else{[void]$held.Add($Keys[$i])}}
    if($sent -ne $Keys.Length){throw 'Incomplete native SendInput batch'}
}
function Batch([uint16[]]$Keys,[bool[]]$Up){Assert-Editor;Send-Batch $Keys $Up}
function Tap([uint16]$Key){Batch @($Key,$Key) @($false,$true);Start-Sleep -Milliseconds 150}
function Haru {foreach($key in @(0x48,0x41,0x52,0x55)){Tap $key}}
function Labels {
    $condition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::Text)
    $all=$root.FindAll([Windows.Automation.TreeScope]::Descendants,$condition)
    $names=@();for($i=0;$i -lt $all.Count;$i++){$names+=,$all.Item($i).Current.Name};return $names
}
function Probe([string]$Name,[string[]]$Arguments) {
    Save "Probe $Name"
    $probe=$null
    try {
        $probe=Start-Process -FilePath $ImeUiaExecutable -ArgumentList $Arguments -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $run "$Name.jsonl") -RedirectStandardError (Join-Path $run "$Name.stderr.log")
        $retainedHandle=$probe.Handle
        if(-not $probe.WaitForExit(20000)){$probe.Kill();[void]$probe.WaitForExit(5000);throw "Read-only UIA probe exceeded 20 seconds: $Name"}
        $probe.WaitForExit()
        if($probe.ExitCode -ne 0){throw "Read-only UIA probe failed ($($probe.ExitCode)): $Name"}
    } finally {if($null -ne $probe){$probe.Dispose()}}
    $rows=@(Get-Content -LiteralPath (Join-Path $run "$Name.jsonl") -Encoding UTF8|ForEach-Object{$_|ConvertFrom-Json})
    [void](Row $rows 'complete')
    return $rows
}
function Row($Rows,[string]$Stage) {
    $found=@($Rows|Where-Object{$_.stage -ceq $Stage})
    if($found.Count -ne 1 -or $found[0].hresult -cne '0x00000000'){throw "Missing/unsupported/failed UIA observation: $Stage"}
    return $found[0]
}
function Active($Snapshot) {
    [void](Row $Snapshot.rows 'TextEdit.GetActiveComposition')
    if(-not (Row $Snapshot.rows 'composition.present').present){return $false}
    $nonempty=(Row $Snapshot.rows 'composition.endpoints').startToEnd -lt 0
    $hasText=(Row $Snapshot.rows 'composition.text').text.Length -gt 0
    if($nonempty -ne $hasText){throw 'Composition range endpoints and text disagree'}
    return $nonempty
}
function Snapshot([string]$Name) {
    Assert-Editor
    $rows=@(Probe $Name @([string]$process.Id,[string]$editor.Current.AutomationId))
    $focus=Row $rows 'focused_element'
    if($focus.pid -ne $process.Id -or $focus.automationId -cne $editor.Current.AutomationId -or -not $focus.hasKeyboardFocus){throw 'Read-only probe observed different editor focus'}
    $value=(Row $rows 'Value.text').text
    if((Row $rows 'document.text').text -cne $value){throw 'Native Value and Text document disagree'}
    $names=@(Labels);$keys=@($names|Where-Object{$_ -match '^Keys: down=\d+ up=\d+ press=\d+ repeats=\d+$'})
    if($keys.Count -ne 1){throw 'Keyboard isolation counter unavailable'}
    $names|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $run "$Name-labels.json") -Encoding UTF8
    $bounds=[Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap=New-Object Drawing.Bitmap([int]$bounds.Width,[int]$bounds.Height);$graphics=[Drawing.Graphics]::FromImage($bitmap)
    try{$graphics.CopyFromScreen($bounds.X,$bounds.Y,0,0,$bitmap.Size);$bitmap.Save((Join-Path $run "$Name.png"))}finally{$graphics.Dispose();$bitmap.Dispose()}
    return [pscustomobject]@{rows=$rows;value=$value;labels=$names;keys=$keys[0]}
}
function Candidate-Window($Snapshot,[string]$Name,$AbsentSnapshot=$null) {
    $old=@();if($null -ne $AbsentSnapshot){$old=@($AbsentSnapshot.rows|Where-Object{$_.stage -ceq 'visible_top_level_window'}|ForEach-Object{$_.hwnd})}
    $windows=@($Snapshot.rows|Where-Object{$_.stage -ceq 'visible_top_level_window' -and $_.hresult -ceq '0x00000000' -and $_.pid -eq $process.Id -and $_.ownerHwnd -eq $process.MainWindowHandle.ToInt64() -and $_.hwnd -notin $old})
    if($windows.Count -gt 8){throw 'Unexpected number of owned candidate windows'}
    $identified=@()
    foreach($window in $windows){
        $tree=@(Probe ($Name+'-tree-'+$window.hwnd) @('--window',[string]$window.hwnd,'128'))
        $nodes=@($tree|Where-Object{$_.stage -ceq 'candidate.element' -and $_.hresult -ceq '0x00000000' -and $_.pid -eq $process.Id})
        $identity=@($nodes|Where-Object{$_.automationId -ceq 'IME_Candidate_Window' -and -not $_.offscreen})
        $items=@($nodes|Where-Object{$_.controlType -eq 50007 -and -not $_.offscreen -and $_.name.Length -gt 0})
        if($identity.Count -eq 1 -and $items.Count -gt 0){$identified+=,[pscustomobject]@{window=$window;nodes=$nodes}}
    }
    Check "$Name Microsoft candidate UI identity and choices observed" ($identified.Count -eq 1) $windows
    return $identified[0]
}
function Click-Native($Element) {
    $process.Refresh()
    if($process.HasExited -or $process.Path -ine $Executable -or $Element.Current.ProcessId -ne $process.Id -or $Element.Current.IsOffscreen){throw 'Native click target unavailable or belongs to another process'}
    [void][PaxImeInput]::SetForegroundWindow($process.MainWindowHandle);Start-Sleep -Milliseconds 150
    if([PaxImeInput]::GetForegroundWindow() -ne $process.MainWindowHandle){throw 'Native click requires fixture foreground'}
    $b=$Element.Current.BoundingRectangle;$point=New-Object PaxImeInput+POINT
    $point.X=[int]($b.Left+[Math]::Min(25,$b.Width/2));$point.Y=[int]($b.Top+$b.Height/2)
    if([PaxImeInput]::GetAncestor([PaxImeInput]::WindowFromPoint($point),2) -ne $process.MainWindowHandle){throw 'Native pointer target is not owned by fixture'}
    if(-not [PaxImeInput]::SetCursorPos($point.X,$point.Y) -or [PaxImeInput]::Click() -ne 2){throw 'Native pointer click failed'}
    Start-Sleep -Milliseconds 250
}
function Compose-Metric([string]$Name,$Target) {
    $script:editor=$Target;Click-Native $editor
    Batch @(0x11,0x41,0x41,0x11) @($false,$false,$true,$true);Tap 0x08;Tap 0x16;Haru;Tap 0x20;Tap 0x20
    $snapshot=Snapshot $Name
    Check "$Name active native composition" (Active $snapshot) $snapshot.value
    $candidate=Candidate-Window $snapshot $Name
    $rectangles=(Row $snapshot.rows 'composition.rectanglesPhysical').rectangles
    Check "$Name one nonempty composition rectangle" ($rectangles.Count -eq 4 -and $rectangles[2] -gt 0 -and $rectangles[3] -gt 0) $rectangles
    $metric=[pscustomobject]@{name=$Name;composition=$rectangles;candidate=$candidate.window.boundsPhysical;editor=(Row $snapshot.rows 'editor').boundsPhysical;value=$snapshot.value}
    $report.metrics+=,$metric;Save $Name;Tap 0x0d
    return $metric
}
function Close-OwnedApp {
    if($null -eq $process){return}
    $process.Refresh()
    if(-not $process.HasExited){
        if(-not $process.CloseMainWindow()){throw 'Owned fixture rejected CloseMainWindow'}
        if(-not $process.WaitForExit(25000)){$process.Kill();[void]$process.WaitForExit(5000);throw 'Owned fixture exceeded 25-second close deadline'}
    }
    $process.WaitForExit();$report.exitCode=$process.ExitCode;$report.hostLog=Host-Log
    Check 'Clean process and export retirement' ($process.ExitCode -eq 0 -and $report.hostLog.Contains('closed exports_drained=true') -and $report.hostLog -notmatch '(?m)^\d+ error ') $process.ExitCode
}
function Restore-Layout {
    if(-not $switchAttempted){return}
    # The fixture must be gone before showing this owned window: activating it
    # earlier would end composition and invalidate the close-during-IME case.
    $restore=New-Object Windows.Forms.Form
    try {
        $restore.Text='Pax IME input-layout restoration';$restore.ShowInTaskbar=$false;$restore.TopMost=$true
        $restore.Size=New-Object Drawing.Size(360,120);$restore.Show();$restore.Activate()
        [Windows.Forms.Application]::DoEvents();[void][PaxImeInput]::SetForegroundWindow($restore.Handle)
        Start-Sleep -Milliseconds 200;[Windows.Forms.Application]::DoEvents()
        if([PaxImeInput]::GetForegroundWindow() -ne $restore.Handle){throw 'Owned restoration window could not gain foreground; layout not restored'}
        $layout=Window-Layout $restore.Handle ([Diagnostics.Process]::GetCurrentProcess().Id)
        if($layout -ne $originalLayout){
            if(($layout.ToInt64() -band 0xffff) -ne 0x0411){throw 'Unexpected input layout during restoration; refusing blind cycling'}
            Send-Batch @(0x5b,0x20,0x20,0x5b) @($false,$false,$true,$true)
            Start-Sleep -Milliseconds 800;[Windows.Forms.Application]::DoEvents()
        }
        $restored=Window-Layout $restore.Handle ([Diagnostics.Process]::GetCurrentProcess().Id)
        Check 'Original English input layout restored on owned window' ($restored -eq $originalLayout) @{before=$originalLayout.ToInt64();after=$restored.ToInt64()}
    } finally {$restore.Close();$restore.Dispose()}
}
try {
    Save 'start'
    Check 'Interactive desktop' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
    Check 'STA diagnostic thread' ([Threading.Thread]::CurrentThread.ApartmentState -eq 'STA') ([string][Threading.Thread]::CurrentThread.ApartmentState)
    $console=[PaxImeInput]::GetConsoleWindow()
    Check 'Hidden runner cannot steal IME foreground' ($console -eq [IntPtr]::Zero -or -not [PaxImeInput]::IsWindowVisible($console)) $console.ToInt64()
    Check 'Physical DPI coordinates' ($previousDpi -ne [IntPtr]::Zero) $previousDpi.ToInt64()
    $profiles=@(Get-WinUserLanguageList);$tips=@($profiles|ForEach-Object{$_.InputMethodTips}|Sort-Object -Unique)
    $japanese='0411:{03B5835F-F03C-411B-9CE2-AA23E1171E36}{A76C93D9-5523-4E90-AAFA-4DB112F9AC76}'
    $report.inputProfiles=$tips
    Check 'Only installed US English and Microsoft Japanese input profiles' ($tips.Count -eq 2 -and $tips -contains '0409:00000409' -and $tips -contains $japanese) $tips
    $originalLayout=[PaxImeInput]::GetKeyboardLayout(0)
    Check 'Initial observed layout is US English' (($originalLayout.ToInt64() -band 0xffff) -eq 0x0409) $originalLayout.ToInt64()
    $report.originalLayout=$originalLayout.ToInt64()
    $existing=@(Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($Executable)) -ErrorAction SilentlyContinue)
    Check 'No competing fixture' ($existing.Count -eq 0) @($existing|ForEach-Object{$_.Id})
    $process=Start-Process -FilePath $Executable -WorkingDirectory (Split-Path $Executable) -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $run 'app.stdout.log') -RedirectStandardError (Join-Path $run 'app.stderr.log')
    $retainedHandle=$process.Handle;$report.pid=$process.Id
    $until=[DateTime]::UtcNow.AddSeconds(60)
    do{Start-Sleep -Milliseconds 200;$process.Refresh();if($process.HasExited){throw 'Fixture exited during launch'}}while(($process.MainWindowHandle -eq [IntPtr]::Zero -or -not (Host-Log).Contains('first_frame_published source=pax-runtime')) -and [DateTime]::UtcNow -lt $until)
    Check 'Actual owned runtime first frame' ($process.Path -ieq $Executable -and (Get-FileHash $process.Path -Algorithm SHA256).Hash -ceq $report.sha256 -and (Host-Log).Contains('first_frame_published source=pax-runtime')) $process.MainWindowHandle.ToInt64()
    $work=[Windows.Forms.Screen]::FromHandle($process.MainWindowHandle).WorkingArea
    Check 'Visible desktop fits fixture' ($work.Width -ge 1040 -and $work.Height -ge 880) $work.ToString()
    Check 'Fixture placed inside desktop' ([PaxImeInput]::MoveWindow($process.MainWindowHandle,$work.Left+20,$work.Top+20,1000,840,$true)) $work.ToString()
    [void][PaxImeInput]::SetForegroundWindow($process.MainWindowHandle);Start-Sleep -Milliseconds 250
    Check 'Owned fixture foreground' ([PaxImeInput]::GetForegroundWindow() -eq $process.MainWindowHandle) $process.Id
    $root=[Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    Check 'Expected native fixture identity' ($root.Current.ProcessId -eq $process.Id -and $root.Current.Name -ceq 'Pax native desktop acceptance') $root.Current.Name
    $condition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::Edit)
    $all=$root.FindAll([Windows.Automation.TreeScope]::Descendants,$condition);$editors=@()
    for($i=0;$i -lt $all.Count;$i++){$editors+=,$all.Item($i)}
    Check 'Two actual native editors' ($editors.Count -eq 2) $editors.Count
    $editor=@($editors|Sort-Object{$_.Current.BoundingRectangle.Top})[0];$report.editor=$editor.Current.AutomationId
    $b=$editor.Current.BoundingRectangle;$point=New-Object PaxImeInput+POINT;$point.X=[int]($b.Left+25);$point.Y=[int]($b.Top+$b.Height/2)
    Check 'Native editor pointer target belongs to fixture' ([PaxImeInput]::GetAncestor([PaxImeInput]::WindowFromPoint($point),2) -eq $process.MainWindowHandle) @($point.X,$point.Y)
    if(-not [PaxImeInput]::SetCursorPos($point.X,$point.Y) -or [PaxImeInput]::Click() -ne 2){throw 'Native editor pointer click failed'}
    Start-Sleep -Milliseconds 200;Assert-Editor
    Check 'Fixture inherited original English layout' ((Window-Layout $process.MainWindowHandle $process.Id) -eq $originalLayout) $originalLayout.ToInt64()
    Batch @(0x11,0x41,0x41,0x11) @($false,$false,$true,$true);Tap 0x08
    Assert-Editor;$switchAttempted=$true
    # One batch guarantees paired Win/Space releases even while the picker opens.
    Send-Batch @(0x5b,0x20,0x20,0x5b) @($false,$false,$true,$true);Start-Sleep -Milliseconds 800
    Assert-Editor;$japaneseLayout=Window-Layout $process.MainWindowHandle $process.Id
    Check 'Guest switched to Japanese input' (($japaneseLayout.ToInt64() -band 0xffff) -eq 0x0411) $japaneseLayout.ToInt64()
    Tap 0x16
    $baseline=Snapshot 'baseline'
    Check 'Empty native baseline without composition' ($baseline.value.Length -eq 0 -and -not (Active $baseline)) $baseline.value
    Haru;$preedit=Snapshot 'preedit'
    $hiragana=([string][char]0x306f)+([string][char]0x308b)
    Check 'Genuine guest Japanese preedit' ((Active $preedit) -and (Row $preedit.rows 'composition.text').text -ceq $hiragana) $preedit.value
    Tap 0x20;$converted=Snapshot 'converted'
    Check 'Space converts active composition' ((Active $converted) -and $converted.value -cne $preedit.value) $converted.value
    Tap 0x20;$candidate=Snapshot 'candidate-before-commit'
    Check 'Genuine active composition before commit' (Active $candidate) $candidate.value
    $observed=Candidate-Window $candidate 'Conversion' $baseline
    $report.candidateWindow=$observed.window;$report.candidateNodes=$observed.nodes
    $selected=$candidate.value;Tap 0x0d;$committed=Snapshot 'committed'
    Check 'Enter ends native composition' (-not (Active $committed)) (Row $committed.rows 'composition.present')
    Check 'Enter preserves selected converted text' ($committed.value -ceq $selected) $committed.value
    $until=[DateTime]::UtcNow.AddSeconds(5)
    do{$names=@(Labels);if($names -ccontains ('Rust received: '+$selected)){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $until)
    Check 'Rust binding receives committed Japanese' ($names -ccontains ('Rust received: '+$selected)) $names
    Batch @(0x11,0x41,0x41,0x11) @($false,$false,$true,$true);$selection=Snapshot 'selection'
    Check 'Native selection covers committed text' ((Row $selection.rows 'selection.0.text').text -ceq $selected) (Row $selection.rows 'selection.0.text')
    Tap 0x23;Haru;$second=Snapshot 'cancel-preedit'
    Check 'Second native preedit active' (Active $second) $second.value
    for($i=0;$i -lt 4;$i++){Tap 0x1b;$cancelled=Snapshot "cancel-step-$i";if(-not (Active $cancelled)){break}}
    Check 'Escape cancels native preedit' (-not (Active $cancelled)) (Row $cancelled.rows 'composition.present')
    Check 'Cancel retains prior committed text' ($cancelled.value -ceq $selected) $cancelled.value
    Check 'Native IME and Escape bypass canvas keyboard handlers' ($baseline.keys -ceq $cancelled.keys) @{before=$baseline.keys;after=$cancelled.keys}
    Assert-Editor;Check 'Native editor retains keyboard focus' $true $editor.Current.AutomationId
    if($CheckGeometry){
        $orderedEditors=@($editors|Sort-Object{$_.Current.BoundingRectangle.Top})
        $translationX=[Math]::Min(150,$work.Width-1040);$translationY=[Math]::Min(100,$work.Height-880)
        Check 'Desktop fits meaningful geometry translation' ($translationX -ge 50 -and $translationY -ge 40) @($translationX,$translationY)
        $original=Compose-Metric 'top-original' $orderedEditors[0]
        Check 'Move owned fixture for candidate geometry' ([PaxImeInput]::MoveWindow($process.MainWindowHandle,$work.Left+20+$translationX,$work.Top+20+$translationY,1000,840,$true)) @($translationX,$translationY)
        Start-Sleep -Milliseconds 250;$moved=Compose-Metric 'top-translated' $orderedEditors[0]
        Check 'Translation actually moves composition' ([Math]::Abs(($moved.composition[0]-$original.composition[0])-$translationX) -le 2 -and [Math]::Abs(($moved.composition[1]-$original.composition[1])-$translationY) -le 2) @{before=$original;after=$moved}
        Check 'Candidate follows translated composition horizontally' ([Math]::Abs(($moved.candidate[0]-$original.candidate[0])-($moved.composition[0]-$original.composition[0])) -le 2) @{before=$original;after=$moved}
        Check 'Candidate follows translated composition vertically' ([Math]::Abs(($moved.candidate[1]-$original.candidate[1])-($moved.composition[1]-$original.composition[1])) -le 2) @{before=$original;after=$moved}
        $buttonCondition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::NameProperty,'Increment')
        $button=$root.FindFirst([Windows.Automation.TreeScope]::Descendants,$buttonCondition)
        if($null -eq $button -or $button.Current.ControlType -ne [Windows.Automation.ControlType]::Button){throw 'Fixture rotation button unavailable'}
        Click-Native $button;Start-Sleep -Milliseconds 1300
        $rotated=Compose-Metric 'top-rotated' $orderedEditors[0]
        Check 'Rotation changes native editor geometry' (($rotated.editor -join ',') -cne ($moved.editor -join ',')) @{before=$moved.editor;after=$rotated.editor}
        Check 'Rotated candidate stays next to composition vertically' ([Math]::Abs($rotated.candidate[1]-($rotated.composition[1]+$rotated.composition[3])) -le 6) $rotated
        $innerBefore=Compose-Metric 'inner-before-scroll' $orderedEditors[1]
        $scrollCondition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ClassNameProperty,'ScrollViewer')
        $scrollers=$root.FindAll([Windows.Automation.TreeScope]::Descendants,$scrollCondition)
        Check 'Two native scrollers for geometry' ($scrollers.Count -eq 2) $scrollers.Count
        for($i=0;$i -lt $scrollers.Count;$i++){
            $pattern=$scrollers.Item($i).GetCurrentPattern([Windows.Automation.ScrollPattern]::Pattern)
            $pattern.SetScrollPercent([Windows.Automation.ScrollPattern]::NoScroll,2);Start-Sleep -Milliseconds 250
            Check 'Native scroller moved before composition' ($pattern.Current.VerticalScrollPercent -gt 1) $pattern.Current.VerticalScrollPercent
        }
        $innerAfter=Compose-Metric 'inner-after-scroll' $orderedEditors[1]
        Check 'Scroll actually moves composition range' ([Math]::Abs($innerAfter.composition[1]-$innerBefore.composition[1]) -gt 5) @{before=$innerBefore;after=$innerAfter}
        Check 'Candidate follows scrolled composition vertically' ([Math]::Abs(($innerAfter.candidate[1]-$innerBefore.candidate[1])-($innerAfter.composition[1]-$innerBefore.composition[1])) -le 2) @{before=$innerBefore;after=$innerAfter}
        Check 'Candidate follows scrolled composition horizontally' ([Math]::Abs(($innerAfter.candidate[0]-$innerBefore.candidate[0])-($innerAfter.composition[0]-$innerBefore.composition[0])) -le 2) @{before=$innerBefore;after=$innerAfter}
    }
    Haru;$closing=Snapshot 'close-active-composition'
    Check 'Actual composition is active immediately before close' (Active $closing) $closing.value
    Check 'Close preedit stays out of canvas keyboard handlers' ($closing.keys -ceq $baseline.keys) $closing.keys
    Assert-Editor;Close-OwnedApp
    $report.result='passed native Microsoft Japanese IME basic and active-composition close checks'
} catch {$report.result='failed';$report.error=$_.Exception.Message;Save 'failed'} finally {
    try {
        foreach($key in @($held)){[void][PaxImeInput]::Keys([uint16[]]@($key),[bool[]]@($true))};$held.Clear()
        if($null -ne $process -and -not $process.HasExited){
            try{Close-OwnedApp}catch{$report.result='failed';$report.closeError=$_.Exception.Message;if(-not $process.HasExited){$process.Kill();[void]$process.WaitForExit(5000)}}
        }
        try{Restore-Layout}catch{$report.result='failed';$report.restoreError=$_.Exception.Message}
    } finally {
        foreach($key in @($held)){[void][PaxImeInput]::Keys([uint16[]]@($key),[bool[]]@($true))}
        if($null -ne $process){$process.Dispose()}
        $env:PAX_WINDOWS_HOST_LOG=$previousLog
        if($previousDpi -ne [IntPtr]::Zero){[void][PaxImeInput]::SetThreadDpiAwarenessContext($previousDpi)}
        $report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save 'finished'
    }
}
Write-Output (Join-Path $run 'result.json')
if($report.result -notlike 'passed*'){exit 1}
