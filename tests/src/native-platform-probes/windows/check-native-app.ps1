param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$NativeUiaExecutable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [string]$FontMetricsExecutable,
    [string]$StagedFontFile,
    [string]$WorkingDirectory,
    [ValidateRange(0,600)][int]$SoakSeconds=0,
    [switch]$SoakWithoutResize
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class NativeAppWindow {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT bounds);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out RECT bounds);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hwnd, int x, int y, int width, int height, bool repaint);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hwnd, StringBuilder text, int length);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr hwnd, StringBuilder text, int length);
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx,dy; public uint mouseData,flags,time; public IntPtr extra; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public MOUSEINPUT mouse; }
    [DllImport("user32.dll", SetLastError=true)] static extern uint SendInput(uint count, INPUT[] inputs, int size);
    public static uint Click() {
        INPUT[] inputs = new INPUT[2]; inputs[0].mouse.flags=2; inputs[1].mouse.flags=4;
        return SendInput(2,inputs,Marshal.SizeOf(typeof(INPUT)));
    }
}
'@
$Executable = (Resolve-Path -LiteralPath $Executable).Path
$NativeUiaExecutable = (Resolve-Path -LiteralPath $NativeUiaExecutable).Path
if ([string]::IsNullOrWhiteSpace($WorkingDirectory)) { $WorkingDirectory=Split-Path $Executable }
$WorkingDirectory=(Resolve-Path -LiteralPath $WorkingDirectory).Path
$run = Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ([Guid]::NewGuid().ToString())
[void](New-Item -ItemType Directory -Path $run -Force)
$logPath = Join-Path $run 'native-host.log'
$oldLog = $env:PAX_WINDOWS_HOST_LOG
$env:PAX_WINDOWS_HOST_LOG = $logPath
$previousDpi = [NativeAppWindow]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$process = $null
$report = [ordered]@{
    result='running; qualification incomplete'; startedAtUtc=[DateTime]::UtcNow.ToString('o');
    executable=$Executable; workingDirectory=$WorkingDirectory; executableSha256=(Get-FileHash $Executable -Algorithm SHA256).Hash;
    nativeUiaExecutable=$NativeUiaExecutable; nativeUiaSha256=(Get-FileHash $NativeUiaExecutable -Algorithm SHA256).Hash;
    checks=@(); nativeCalls=@(); screenshots=@(); process=$null;
    presentationReadback='screenshots are acceptance diagnostics; production presentation stays on GPU';
    managedTextRangeGetText='not exercised; prior Server 2022 managed marshaling failure remains separate';
    ime='not qualified by keyboard injection'; bundledFont='resource path and screenshot only; resolved glyph-face qualification remains separate'
}
function Checkpoint([string]$Name) {
    $report.checkpoint=$Name; $report.checkpointAtUtc=[DateTime]::UtcNow.ToString('o')
    $temp=Join-Path $run 'result.tmp.json'; $destination=Join-Path $run 'result.json'
    $report|ConvertTo-Json -Depth 12|Set-Content -LiteralPath $temp -Encoding UTF8
    Move-Item -LiteralPath $temp -Destination $destination -Force
}
function Check([string]$Name,[bool]$Passed,$Detail) {
    $report.checks += [ordered]@{name=$Name;passed=$Passed;detail=$Detail}
    Checkpoint "completed: $Name"
    if( -not $Passed){throw "$Name failed: $Detail"}
}
function Log-Text {
    if( -not (Test-Path -LiteralPath $logPath)){return ''}
    return [IO.File]::ReadAllText($logPath)
}
function Find-Name([string]$Name) {
    Checkpoint "UIA FindFirst name: $Name"
    $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,$Name)
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
}
function Wait-Name([string]$Name) {
    $deadline=[DateTime]::UtcNow.AddSeconds(10)
    do {$item=Find-Name $Name;if($null -ne $item){return $item};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
    throw "Native content did not appear: $Name"
}
function Controls-OfType($Type) {
    Checkpoint "UIA FindAll type: $Type"
    $condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$Type)
    $result=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
    $items=@();for($i=0;$i -lt $result.Count;$i++){$items+=,$result.Item($i)}
    return $items
}
function Foreground {
    $hwnd=$process.MainWindowHandle
    $requested=[NativeAppWindow]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 150
    $before=[NativeAppWindow]::GetForegroundWindow()
    $title=New-Object System.Text.StringBuilder 512;$class=New-Object System.Text.StringBuilder 256
    [void][NativeAppWindow]::GetWindowTextW($hwnd,$title,$title.Capacity)
    [void][NativeAppWindow]::GetClassNameW($hwnd,$class,$class.Capacity)
    [uint32]$owner=0;[void][NativeAppWindow]::GetWindowThreadProcessId($hwnd,[ref]$owner)
    $detail=@{requested=$requested;target=$hwnd.ToInt64();targetPid=$owner;title=$title.ToString();class=$class.ToString();foregroundBefore=$before.ToInt64();pointerClick=$false}
    Check 'Foreground target is the fixture window' ($owner -eq $process.Id -and $title.ToString() -ceq 'Pax native desktop acceptance') $detail
    if($before -ne $hwnd){
        # A scheduled checker is not necessarily entitled to steal foreground.
        # Exercise a real title-bar click only after the visible hit belongs to
        # this exact application window; do not click an occluding application.
        $bounds=New-Object NativeAppWindow+RECT
        $read=[NativeAppWindow]::GetWindowRect($hwnd,[ref]$bounds)
        $scale=[NativeAppWindow]::GetDpiForWindow($hwnd)/96.0
        $point=New-Object NativeAppWindow+POINT
        $point.X=$bounds.Left+[int][Math]::Round([Math]::Min(100*$scale,($bounds.Right-$bounds.Left)/2))
        $point.Y=$bounds.Top+[int][Math]::Round(16*$scale)
        $hit=[NativeAppWindow]::WindowFromPoint($point)
        $hitRoot=[NativeAppWindow]::GetAncestor($hit,2)
        $detail.click=@($point.X,$point.Y);$detail.hit=$hit.ToInt64();$detail.hitRoot=$hitRoot.ToInt64()
        Check 'Foreground title-bar hit belongs to fixture' ($read -and $scale -gt 0 -and $hitRoot -eq $hwnd) $detail
        $moved=[NativeAppWindow]::SetCursorPos($point.X,$point.Y)
        $actual=New-Object NativeAppWindow+POINT;$positioned=[NativeAppWindow]::GetCursorPos([ref]$actual)
        Check 'Foreground pointer reached observed title bar' ($moved -and $positioned -and $actual.X -eq $point.X -and $actual.Y -eq $point.Y) @($actual.X,$actual.Y)
        $sent=[NativeAppWindow]::Click();$detail.pointerClick=$true;$detail.sent=$sent
        Check 'Foreground title-bar click injected completely' ($sent -eq 2) $sent
        Start-Sleep -Milliseconds 200
    }
    $detail.foregroundAfter=[NativeAppWindow]::GetForegroundWindow().ToInt64()
    Check 'Application owns foreground' ($detail.foregroundAfter -eq $hwnd.ToInt64()) $detail
}
function Move-App([int]$Width,[int]$Height,[int]$Inset) {
    $work=[System.Windows.Forms.Screen]::FromHandle($process.MainWindowHandle).WorkingArea
    $w=[Math]::Min($Width,$work.Width-2*$Inset);$h=[Math]::Min($Height,$work.Height-2*$Inset)
    $ok=[NativeAppWindow]::MoveWindow($process.MainWindowHandle,$work.Left+$Inset,$work.Top+$Inset,$w,$h,$true)
    $bounds=New-Object NativeAppWindow+RECT
    $read=[NativeAppWindow]::GetWindowRect($process.MainWindowHandle,[ref]$bounds)
    Check 'MoveWindow and actual visible bounds' ($ok -and $read -and $bounds.Left -ge $work.Left -and $bounds.Top -ge $work.Top -and $bounds.Right -le $work.Right -and $bounds.Bottom -le $work.Bottom) @{requested=@($w,$h);actual=@($bounds.Left,$bounds.Top,$bounds.Right,$bounds.Bottom)}
}
function Capture([string]$Name) {
    Checkpoint "composed screenshot: $Name"
    Foreground
    $bounds=New-Object NativeAppWindow+RECT
    if( -not [NativeAppWindow]::GetWindowRect($process.MainWindowHandle,[ref]$bounds)){throw 'Missing window bounds'}
    $bitmap=New-Object System.Drawing.Bitmap(($bounds.Right-$bounds.Left),($bounds.Bottom-$bounds.Top))
    $graphics=[Drawing.Graphics]::FromImage($bitmap)
    try {$graphics.CopyFromScreen($bounds.Left,$bounds.Top,0,0,$bitmap.Size);$path=Join-Path $run $Name;$bitmap.Save($path,[Drawing.Imaging.ImageFormat]::Png)}finally{$graphics.Dispose()}
    $report.screenshots+=@{path=$path;left=$bounds.Left;top=$bounds.Top;width=$bitmap.Width;height=$bitmap.Height}
    return @{bitmap=$bitmap;left=$bounds.Left;top=$bounds.Top}
}
function Native-Selection([string]$Phase,[bool]$Selected) {
    $directory=Join-Path $run $Phase;[void](New-Item -ItemType Directory -Path $directory)
    $out=Join-Path $directory 'stdout.jsonl';$err=Join-Path $directory 'stderr.log'
    $id=$editor.Current.AutomationId
    Check 'Native editor has a stable runtime automation ID' ($id -match '^pax-node-\d+$') $id
    $call=[ordered]@{phase=$Phase;automationId=$id;targetPid=$process.Id;status='starting';directory=$directory}
    $report.nativeCalls+=,$call
    Checkpoint "native COM selection $Phase; supervised 20 second deadline"
    $helper=Start-Process -FilePath $NativeUiaExecutable -ArgumentList @([string]$process.Id,'-1',$id) -PassThru -NoNewWindow -RedirectStandardOutput $out -RedirectStandardError $err
    $retainedHandle=$helper.Handle
    try {
        if( -not $helper.WaitForExit(20000)){$helper.Kill();[void]$helper.WaitForExit(5000);throw 'Native UIA helper exceeded its deadline'}
        $exit=$helper.ExitCode
    } finally {$helper.Dispose()}
    Check "Native UIA $Phase clean helper exit" ($exit -eq 0) $exit
    $stages=@{};$last=''
    foreach($line in Get-Content -LiteralPath $out){
        if([string]::IsNullOrWhiteSpace($line)){continue}
        $value=$line|ConvertFrom-Json
        if($value.PSObject.Properties.Name -notcontains 'stage'){continue}
        if($stages.ContainsKey($value.stage)){throw "Duplicate native UIA stage: $($value.stage)"}
        $stages[$value.stage]=$value;$last=$value.stage
        if( -not $value.succeeded -and  -not ($value.stage -eq 'QueryInterface_IUIAutomation2' -and $value.hresult -eq '0x80004002')){throw "Native UIA failed: $line"}
    }
    $required=@('CoInitializeEx_MTA','target_window','CoCreateInstance_CUIAutomation','QueryInterface_IUIAutomation2','ElementFromHandle','CreatePropertyCondition_native_editor','FindFirst_native_editor','editor_process_identity','GetCurrentPatternAs_Value','ValuePattern_get_CurrentValue','GetCurrentPatternAs_Text','TextPattern_get_DocumentRange','TextPattern_GetSelection','selection_range_count','selection_GetElement_0','document_start_to_end','selection_start_to_end','selection_start_to_document_start','selection_end_to_document_end','selection_GetText','document_GetText','complete')
    foreach($name in $required){if( -not $stages.ContainsKey($name)){throw "Missing native UIA stage: $name"}}
    if($stages.QueryInterface_IUIAutomation2.succeeded){foreach($name in @('put_ConnectionTimeout_5000','put_TransactionTimeout_5000')){if( -not $stages.ContainsKey($name)){throw "Missing UIA timeout: $name"}}}
    $identity=$stages.target_window.pid -eq $process.Id -and $stages.target_window.hwnd -eq $process.MainWindowHandle.ToInt64() -and $stages.editor_process_identity.actualPid -eq $process.Id -and $stages.CreatePropertyCondition_native_editor.automationId -ceq $id
    $complete=$last -eq 'complete' -and $stages.complete.readOnly -eq $true -and $stages.complete.semanticAssertions -eq $false
    $content=$stages.ValuePattern_get_CurrentValue.text -ceq $fixtureText -and $stages.document_GetText.text -ceq $fixtureText -and $stages.selection_GetText.maxLength -eq -1 -and $stages.document_GetText.maxLength -eq -1
    Check "Native UIA $Phase exact document and target" ($identity -and $complete -and $content -and $stages.selection_range_count.count -eq 1 -and $stages.document_start_to_end.ordering -lt 0) $directory
    if($Selected){
        $selection=$stages.selection_GetText.text -ceq $fixtureText -and $stages.selection_start_to_end.ordering -lt 0 -and $stages.selection_start_to_document_start.ordering -eq 0 -and $stages.selection_end_to_document_end.ordering -eq 0
    } else {
        $selection=$stages.selection_GetText.text -ceq '' -and $stages.selection_start_to_end.ordering -eq 0 -and $stages.selection_start_to_document_start.ordering -eq 0 -and $stages.selection_end_to_document_end.ordering -lt 0
    }
    Check "Native UIA $Phase selection semantics" $selection @{selected=$Selected;text=$stages.selection_GetText.text}
    $call.status='passed';Checkpoint "native COM selection $Phase passed"
}
function Check-BundledFont {
    if ([string]::IsNullOrWhiteSpace($FontMetricsExecutable)) { return }
    $helperPath=(Resolve-Path -LiteralPath $FontMetricsExecutable).Path
    $exeRoot=Split-Path $Executable
    if ([string]::IsNullOrWhiteSpace($StagedFontFile)) {
        $candidates=@()
        foreach($directory in @((Join-Path $exeRoot 'pax-fonts'),(Join-Path $exeRoot 'assets'))) {
            if(Test-Path -LiteralPath $directory) { $candidates+=@(Get-ChildItem -LiteralPath $directory -Recurse -Filter Manrope.ttf -File) }
        }
        Check 'One unambiguous staged font file' ($candidates.Count -eq 1) @($candidates|ForEach-Object{$_.FullName})
        $font=$candidates[0].FullName
    } else { $font=(Resolve-Path -LiteralPath $StagedFontFile).Path }
    Check 'Font diagnostic uses executable-relative staged asset' ($font.StartsWith($exeRoot.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)) $font
    $label=Wait-Name 'Bundled Manrope: @@@@@@@@'
    $id=$label.Current.AutomationId
    Check 'Bundled font label is visible and has a runtime ID' ( -not $label.Current.IsOffscreen -and $id -match '^pax-node-\d+$') $id
    $stdout=Join-Path $run 'font-metrics.jsonl';$stderr=Join-Path $run 'font-metrics.stderr.log'
    $report.bundledFont='native glyph-metric comparison running; not qualified yet'
    Checkpoint 'native font metric helper; supervised 20 second deadline'
    $helper=Start-Process -FilePath $helperPath -ArgumentList @([string]$process.Id,$id,('"'+$font+'"')) -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    $retainedFontHandle=$helper.Handle
    try {
        if( -not $helper.WaitForExit(20000)){$helper.Kill();[void]$helper.WaitForExit(5000);throw 'Font metric helper exceeded its deadline'}
        $exit=$helper.ExitCode
    } finally { $helper.Dispose() }
    $complete=@(Get-Content -LiteralPath $stdout|ForEach-Object{$_|ConvertFrom-Json}|Where-Object{$_.PSObject.Properties.Name -contains 'stage' -and $_.stage -eq 'complete'})
    Check 'Font metric helper completed successfully' ($exit -eq 0 -and $complete.Count -eq 1) @{exit=$exit;output=$stdout}
    $metrics=$complete[0]
    $report.fontMetrics=$metrics;$report.stagedFontSha256=(Get-FileHash -LiteralPath $font -Algorithm SHA256).Hash
    $correct=$metrics.pid -eq $process.Id -and $metrics.hwnd -eq $process.MainWindowHandle.ToInt64() -and $metrics.automationId -ceq $id -and
        $metrics.fontFile -ceq $font -and $metrics.text -ceq 'Bundled Manrope: @@@@@@@@' -and $metrics.qualified -eq $true -and $metrics.negativeControlsDistinct -eq $true
    Check 'Native glyph metrics match bundled Manrope and exclude system fallbacks' $correct $metrics
    $report.bundledFont='qualified: native text-range metrics match exact staged Manrope font with distinct Arial and Segoe UI negative controls'
}
try {
    Checkpoint 'start'
    Check 'Interactive desktop session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
    Check 'Physical DPI coordinate mode' ($previousDpi -ne [IntPtr]::Zero) $previousDpi.ToInt64()
    $name=[IO.Path]::GetFileNameWithoutExtension($Executable)
    $existing=@(Get-Process -Name $name -ErrorAction SilentlyContinue)
    Check 'No competing application process' ($existing.Count -eq 0) @($existing|ForEach-Object{$_.Id})
    $process=Start-Process -FilePath $Executable -WorkingDirectory $WorkingDirectory -PassThru -RedirectStandardOutput (Join-Path $run 'app.stdout.log') -RedirectStandardError (Join-Path $run 'app.stderr.log')
    $retainedAppHandle=$process.Handle;$report.process=$process.Id
    $deadline=[DateTime]::UtcNow.AddSeconds(60)
    do {
        Start-Sleep -Milliseconds 200;$process.Refresh()
        if($process.HasExited){throw "App exited before first frame: $($process.ExitCode)"}
    }while(($process.MainWindowHandle -eq [IntPtr]::Zero -or  -not (Log-Text).Contains('first_frame_published source=pax-runtime')) -and [DateTime]::UtcNow -lt $deadline)
    Check 'Real Pax runtime first GPU frame' ((Log-Text).Contains('first_frame_published source=pax-runtime')) $process.MainWindowHandle.ToInt64()
    $initialized=[regex]::Match((Log-Text),'(?m)^\d+ initialized_gpu=(.+)\r?$')
    Check 'GPU diagnostics present' $initialized.Success 'Initialization JSON from the real cartridge backend'
    $gpu=$initialized.Groups[1].Value|ConvertFrom-Json
    Check 'Hardware DX12 admission without presentation readback' ($gpu.transport -eq 'dx12-composition-v1' -and $gpu.deviceType -in @('DiscreteGpu','IntegratedGpu') -and $gpu.cpuPresentationReadback -eq $false) $gpu
    Move-App 1000 840 20;Foreground
    Checkpoint 'UIA root'
    $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $title=Wait-Name 'Initialized by user-owned main'
    Check 'User-owned Rust main startup reached Pax template' ($null -ne $title) $title.Current.Name
    Check-BundledFont
    [void](Wait-Name 'Count: 0')
    $button=Wait-Name 'Increment'
    Check 'Actual native button peer' ($button.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button) $button.Current.ClassName
    Checkpoint 'UIA Invoke native button'
    $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    [void](Wait-Name 'Count: 1');Check 'Native button dispatch reached Rust handler' $true 'Count: 0 -> Count: 1'
    $checks=@(Controls-OfType ([System.Windows.Automation.ControlType]::CheckBox))
    Check 'Exactly one native checkbox' ($checks.Count -eq 1) $checks.Count
    Checkpoint 'UIA Toggle checkbox'
    $checks[0].GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
    [void](Wait-Name 'Checked: yes');Check 'Native checkbox updates Pax binding' $true 'Checked: yes'
    $editors=@(Controls-OfType ([System.Windows.Automation.ControlType]::Edit)|Sort-Object {$_.Current.BoundingRectangle.Top})
    Check 'Two actual native editors' ($editors.Count -eq 2) @($editors|ForEach-Object{$_.Current.ClassName})
    $editor=$editors[0];$fixtureText='Pax application input 1038'
    Check 'Primary editor is visible' ( -not $editor.Current.IsOffscreen) $editor.Current.BoundingRectangle.ToString()
    Checkpoint 'Native editor focus and keyboard input'
    $editor.SetFocus();[Windows.Forms.SendKeys]::SendWait('^a');[Windows.Forms.SendKeys]::SendWait($fixtureText)
    [void](Wait-Name "Rust received: $fixtureText")
    $value=$editor.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $second=$editors[1].GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    Check 'Keyboard input reaches Rust event and both bound editors' ($value.Current.Value -ceq $fixtureText -and $second.Current.Value -ceq $fixtureText) $value.Current.Value
    [Windows.Forms.SendKeys]::SendWait('^{HOME}');Start-Sleep -Milliseconds 100;Native-Selection 'collapsed-negative-control' $false
    [Windows.Forms.SendKeys]::SendWait('^a');Start-Sleep -Milliseconds 100;Native-Selection 'selection-before-repaint' $true
    $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    [void](Wait-Name 'Count: 2');Start-Sleep -Seconds 2
    Native-Selection 'selection-after-affine-animation' $true
    $shot=Capture 'selected-composed.png';$shot.bitmap.Dispose()
    $scrollers=@(Controls-OfType ([System.Windows.Automation.ControlType]::Pane)|Where-Object{$_.Current.ClassName -eq 'ScrollViewer'})
    if($scrollers.Count -ne 2){
        $all=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
        $scrollers=@();for($i=0;$i -lt $all.Count;$i++){$item=$all.Item($i);if($item.Current.ClassName -eq 'ScrollViewer'){$scrollers+=,$item}}
    }
    Check 'Two nested native ScrollViewer peers' ($scrollers.Count -eq 2) @($scrollers|ForEach-Object{$_.Current.AutomationId})
    foreach($scroller in $scrollers){
        Checkpoint 'Native scroll pattern'
        $pattern=$scroller.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
        $pattern.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll,45)
        Start-Sleep -Milliseconds 350
        Check 'Native vertical scroll moved' ($pattern.Current.VerticalScrollPercent -gt 1) $pattern.Current.VerticalScrollPercent
        $pattern.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll,0);Start-Sleep -Milliseconds 200
    }
    Move-App 940 780 30;Start-Sleep -Milliseconds 600
    Check 'Editor identity and text survive resize and scrolling' ($value.Current.Value -ceq $fixtureText -and $second.Current.Value -ceq $fixtureText) $editor.Current.AutomationId
    $shot=Capture 'resized-composed.png'
    try {
        $client=New-Object NativeAppWindow+RECT;$origin=New-Object NativeAppWindow+POINT
        $ok=[NativeAppWindow]::GetClientRect($process.MainWindowHandle,[ref]$client) -and [NativeAppWindow]::ClientToScreen($process.MainWindowHandle,[ref]$origin)
        Check 'Composed image sample geometry available' $ok @($origin.X,$origin.Y,$client.Right,$client.Bottom)
        $scale=[NativeAppWindow]::GetDpiForWindow($process.MainWindowHandle)/96.0
        # Fixture smoke.png contains 8-pixel checker cells, rendered at 48 DIP.
        # Sample cell centers away from filtering edges; orange has alpha 128/255.
        foreach($sample in @(@{name='opaque blue image';x=18;y=6;rgb=@(30,120,220)},@{name='half-alpha orange image over white';x=6;y=6;rgb=@(247,212,142)})){
            $x=[int][Math]::Round($origin.X+$client.Right-76*$scale+$sample.x*$scale-$shot.left)
            $y=[int][Math]::Round($origin.Y+(20+$sample.y)*$scale-$shot.top)
            $pixel=$shot.bitmap.GetPixel($x,$y);$actual=@([int]$pixel.R,[int]$pixel.G,[int]$pixel.B)
            $match=$true;for($c=0;$c -lt 3;$c++){$match=$match -and [Math]::Abs($actual[$c]-$sample.rgb[$c]) -le 5}
            Check "Final composed $($sample.name) pixel" $match @{position=@($x,$y);actual=$actual;expected=$sample.rgb;tolerance=5}
        }
    }finally{$shot.bitmap.Dispose()}
    if ($SoakSeconds -gt 0) {
        $workload=if($SoakWithoutResize){'retarget native/vector affine animation every 500ms; fixed window size'}else{'retarget native/vector affine animation every 500ms; resize every 10s'}
        $report.soak=[ordered]@{seconds=$SoakSeconds;measurement='host publication cadence and process resources; not display/DCV FPS';workload=$workload;samples=@()}
        $started=[DateTime]::UtcNow;$until=$started.AddSeconds($SoakSeconds);$iteration=0
        do {
            $process.Refresh()
            if($process.HasExited){throw 'Application exited during bounded resize soak'}
            if(-not $SoakWithoutResize -and $iteration % 20 -eq 0){
                $wide=($iteration % 40 -eq 0)
                Move-App $(if($wide){1000}else{940}) $(if($wide){840}else{780}) 30
            }
            $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
            if($iteration % 4 -eq 0){
                $report.soak.samples+=@{seconds=([DateTime]::UtcNow-$started).TotalSeconds;workingSetBytes=$process.WorkingSet64;privateBytes=$process.PrivateMemorySize64;cpuSeconds=$process.TotalProcessorTime.TotalSeconds;handles=$process.HandleCount}
                Checkpoint 'bounded resize soak'
            }
            Start-Sleep -Milliseconds 500;$iteration++
        }while([DateTime]::UtcNow -lt $until)
        Check 'Native editor identity and binding survive bounded resize soak' ($value.Current.Value -ceq $fixtureText -and $second.Current.Value -ceq $fixtureText) $editor.Current.AutomationId
        $states=@([regex]::Matches((Log-Text),'(?m)^\d+ gpu=(.+)\r?$') | ForEach-Object { $_.Groups[1].Value | ConvertFrom-Json })
        $outside=@($states | Where-Object { $_.pendingFrames -gt 3 -or $_.allocatedImages -gt $_.imageCapacity -or $_.allocatedBytes -gt $_.exportByteBudget -or $_.cpuPresentationReadback })
        Check 'Observed GPU export resources stay inside declared bounds' ($states.Count -gt 0 -and $outside.Count -eq 0) @{observations=$states.Count;violations=$outside}
        $publications=@([regex]::Matches((Log-Text),'(?m)^(\d+) frame=(\d+) pending=(\d+) submitted=(\d+) retired=(\d+) peak=(\d+) backpressure=(\d+)'))
        if($publications.Count -ge 2){
            $first=$publications[0];$last=$publications[-1]
            $seconds=([double]$last.Groups[1].Value-[double]$first.Groups[1].Value)/1000.0
            $report.soak.publicationCadence=@{seconds=$seconds;frames=([long]$last.Groups[2].Value-[long]$first.Groups[2].Value);framesPerSecond=([double]$last.Groups[2].Value-[double]$first.Groups[2].Value)/$seconds;peakInFlight=[int]$last.Groups[6].Value;backpressure=[long]$last.Groups[7].Value}
        }
        $shot=Capture 'soak-final.png';$shot.bitmap.Dispose()
    }
    Check 'No host error before close' ( -not ((Log-Text) -match '(?m)^\d+ error ')) 'Native host log'
    $report.result='interactive checks passed; close pending'
} catch {
    $report.result='failed';$report.error=$_.Exception.Message
    if($null -ne $process -and  -not $process.HasExited -and $process.MainWindowHandle -ne [IntPtr]::Zero){try{$shot=Capture 'failure.png';$shot.bitmap.Dispose()}catch{}}
} finally {
    if($null -ne $process){
        Checkpoint 'close owned application; asynchronous retirement deadline'
        if( -not $process.HasExited){[void]$process.CloseMainWindow()}
        if( -not $process.WaitForExit(25000)){
            $report.forcedTermination=$true;$report.result='failed';$report.closeError='Application did not close in 25 seconds; diagnostic process terminated'
            $process.Kill();[void]$process.WaitForExit(5000)
        }
        $report.exitCode=$process.ExitCode
        $text=Log-Text;$report.hostLog=$text
        $closed=$text.Contains('closed exports_drained=true') -and $process.ExitCode -eq 0
        $report.checks+=@{name='Clean process exit after asynchronous export retirement';passed=$closed;detail=$process.ExitCode}
        $drained=[regex]::Match($text,'(?m)^\d+ closed_gpu=(.+)\r?$')
        $bounded=$false
        if($drained.Success){
            $state=$drained.Groups[1].Value|ConvertFrom-Json
            $report.closedGpu=$state
            $bounded=$state.pendingFrames -eq 0 -and $state.pendingBytes -eq 0 -and $state.allocatedImages -eq 0 -and $state.allocatedBytes -eq 0 -and
                $state.published -eq $state.retired -and $state.published -ge 30 -and $state.capacity -eq 3 -and $state.closing -eq $true
        }
        $report.checks+=@{name='Continuous frame delivery drained all bounded exports';passed=$bounded;detail='At least 30 published images; all retired; no live export allocation on close'}
        if( -not $closed -or  -not $bounded){$report.result='failed'}
        elseif($report.result -eq 'interactive checks passed; close pending'){$report.result='passed scripted native application checks; explicit exclusions remain'}
        $process.Dispose()
    }
    $env:PAX_WINDOWS_HOST_LOG=$oldLog
    if($previousDpi -ne [IntPtr]::Zero){[void][NativeAppWindow]::SetThreadDpiAwarenessContext($previousDpi)}
    Checkpoint 'finished'
}
Write-Output (Join-Path $run 'result.json')
if($report.result -notlike 'passed*'){exit 1}
