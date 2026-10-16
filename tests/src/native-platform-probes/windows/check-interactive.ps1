param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'ARM64',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [ValidateSet('d3d11', 'dx12', 'pax')][string]$Producer = 'd3d11',
    [ValidateSet('GetText', 'Endpoints', 'Native')][string]$SelectionReadback = 'GetText',
    [string]$NativeUiaExecutable,
    [switch]$AllowWarp,
    [switch]$KeepOpen
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ProbeWindow {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT bounds);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out RECT bounds);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll", SetLastError = true)] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extraInfo);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hwnd, int x, int y, int width, int height, bool repaint);
}
'@
# Use physical pixels for both MoveWindow and screenshot bounds at mixed DPI.
$previousDpiContext = [ProbeWindow]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$runDirectory = Join-Path $PSScriptRoot "out\$Architecture\$Configuration"
$report = [ordered]@{
    runId = [Guid]::NewGuid().ToString()
    startedAtUtc = [DateTime]::UtcNow.ToString('o')
    checkerProcess = $PID
    checkerApartment = [Threading.Thread]::CurrentThread.GetApartmentState().ToString()
    result = 'running; qualification incomplete'
    architecture = $Architecture
    configuration = $Configuration
    producer = $Producer
    selectionReadback = $SelectionReadback
    textRangeGetTextExercised = $false
    textRangeGetTextQualified = $false
    textRangeGetTextStatus = $(if ($SelectionReadback -ne 'GetText') {
        "managed GetText not exercised; not qualified in explicit $SelectionReadback mode"
    } else { 'not yet exercised; qualification incomplete' })
    nativeGetTextQualified = $false
    nativeGetTextExercised = $false
    nativeUiaCalls = @()
    allowWarp = [bool]$AllowWarp
    process = $null
    checks = @()
    windowMoves = @()
    pointerAttempts = @()
    ime = 'not tested; keyboard text injection does not establish IME composition support'
    gpu = 'see composition-probe.log adapter and device type; VM evidence only'
}
$process = $null
$scriptedChecksPassed = $false
$logPath = Join-Path $runDirectory 'composition-probe.log'
$logOffset = if (Test-Path $logPath) { (Get-Item $logPath).Length } else { 0 }

function Save-Report {
    $destination = Join-Path $PSScriptRoot 'interactive-result.json'
    $temporary = "$destination.$PID.tmp"
    $report | ConvertTo-Json -Depth 8 | Set-Content $temporary -Encoding UTF8
    Move-Item -LiteralPath $temporary -Destination $destination -Force
}

function Checkpoint([string]$Operation) {
    $report.checkpoint = $Operation
    $report.checkpointAtUtc = [DateTime]::UtcNow.ToString('o')
    Save-Report
}

function Record-Check([string]$Name, [bool]$Passed, [string]$Detail) {
    $report.checks += [ordered]@{ name = $Name; passed = $Passed; detail = $Detail }
    Checkpoint "completed check: $Name"
    if (-not $Passed) { throw "$Name failed: $Detail" }
}

function Find-Control($Root, [string]$AutomationId) {
    Checkpoint "UIA FindFirst: $AutomationId"
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $AutomationId)
    $result = $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
    if ($null -eq $result) { throw "Control not found: $AutomationId" }
    return $result
}

function Bounds-Report($Bounds) {
    return [ordered]@{ left = $Bounds.Left; top = $Bounds.Top; right = $Bounds.Right; bottom = $Bounds.Bottom;
        width = $Bounds.Width; height = $Bounds.Height }
}

function Test-ValidBounds($Bounds) {
    return (-not [double]::IsNaN($Bounds.Left) -and -not [double]::IsInfinity($Bounds.Left) -and
        -not [double]::IsNaN($Bounds.Top) -and -not [double]::IsInfinity($Bounds.Top) -and
        $Bounds.Width -gt 0 -and $Bounds.Height -gt 0 -and
        -not [double]::IsInfinity($Bounds.Width) -and -not [double]::IsInfinity($Bounds.Height))
}

function Test-PointInside($Bounds, [double]$X, [double]$Y) {
    return ((Test-ValidBounds $Bounds) -and $X -gt $Bounds.Left -and $X -lt $Bounds.Right -and
        $Y -gt $Bounds.Top -and $Y -lt $Bounds.Bottom)
}

function Get-ProbeClientBounds {
    $bounds = New-Object ProbeWindow+RECT
    $origin = New-Object ProbeWindow+POINT
    if (-not [ProbeWindow]::GetClientRect($process.MainWindowHandle, [ref]$bounds) -or
        -not [ProbeWindow]::ClientToScreen($process.MainWindowHandle, [ref]$origin)) {
        throw 'Cannot obtain the probe client bounds in physical screen pixels.'
    }
    return [System.Windows.Rect]::new($origin.X, $origin.Y, $bounds.Right - $bounds.Left, $bounds.Bottom - $bounds.Top)
}

function Move-ProbeWindow([int]$Width, [int]$Height, [int]$Inset) {
    $work = [System.Windows.Forms.Screen]::FromHandle($process.MainWindowHandle).WorkingArea
    # Keep the requested window on the actual monitor, including small DCV desktops.
    $width = [Math]::Min($Width, $work.Width - 2 * $Inset)
    $height = [Math]::Min($Height, $work.Height - 2 * $Inset)
    if ($width -le 0 -or $height -le 0) { throw 'Desktop working area cannot contain the probe window.' }
    $moved = [ProbeWindow]::MoveWindow($process.MainWindowHandle, $work.Left + $Inset, $work.Top + $Inset, $width, $height, $true)
    $actual = New-Object ProbeWindow+RECT
    $read = [ProbeWindow]::GetWindowRect($process.MainWindowHandle, [ref]$actual)
    $report.windowMoves += [ordered]@{ apiSucceeded = $moved; boundsRead = $read; workingArea = (Bounds-Report $work);
        requested = @{ left = $work.Left + $Inset; top = $work.Top + $Inset; width = $width; height = $height };
        actual = @{ left = $actual.Left; top = $actual.Top; right = $actual.Right; bottom = $actual.Bottom } }
    Record-Check 'Window move succeeded within desktop' ($moved -and $read -and $actual.Left -ge $work.Left -and
        $actual.Top -ge $work.Top -and $actual.Right -le $work.Right -and $actual.Bottom -le $work.Bottom) "requested=${width}x${height}; API=$moved"
}

function Read-RunLog {
    if (-not (Test-Path $logPath)) { return '' }
    $stream = [System.IO.File]::Open($logPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    try {
        [void]$stream.Seek($logOffset, [System.IO.SeekOrigin]::Begin)
        $reader = New-Object System.IO.StreamReader($stream)
        try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
    } finally { $stream.Dispose() }
}

function Read-NativeFrameCheckpoint {
    $runLog = Read-RunLog
    # Match the native timer's frame/selection records, not Rust producer logs.
    $samples = [regex]::Matches($runLog, '(?m)^\d+ frame=(\d+) selection=(\d+)/(\d+)\r?$')
    if ($samples.Count -gt 0) {
        $sample = $samples[$samples.Count - 1]
        return [ordered]@{ frame = [long]$sample.Groups[1].Value; selectionStart = [int]$sample.Groups[2].Value;
            selectionLength = [int]$sample.Groups[3].Value; source = 'native timer sample (logged every 60 frames)' }
    }
    return [ordered]@{ frame = $(if ($runLog.Contains("first_frame_presented producer=$Producer")) { 1 } else { 0 });
        selectionStart = $null; selectionLength = $null; source = 'first-frame marker; no periodic sample yet' }
}

function Read-SelectionEndpoints($TextPattern, [string]$Phase) {
    Checkpoint "UIA endpoints ${Phase}: DocumentRange"
    $document = $TextPattern.DocumentRange
    Checkpoint "UIA endpoints ${Phase}: fresh GetSelection"
    $ranges = $TextPattern.GetSelection()
    Record-Check "Endpoint ${Phase} exposes exactly one selection range" ($ranges.Count -eq 1) "ranges=$($ranges.Count)"
    $start = [System.Windows.Automation.Text.TextPatternRangeEndpoint]::Start
    $end = [System.Windows.Automation.Text.TextPatternRangeEndpoint]::End
    $selected = $ranges[0]
    Checkpoint "UIA endpoints ${Phase}: CompareEndpoints"
    # CompareEndpoints returns ordering, not a character count. Keep the strict
    # document-span contract; do not normalize an unexpected provider endpoint.
    return [ordered]@{
        documentStartToEnd = $document.CompareEndpoints($start, $document, $end)
        selectionStartToEnd = $selected.CompareEndpoints($start, $selected, $end)
        selectionStartToDocumentStart = $selected.CompareEndpoints($start, $document, $start)
        selectionEndToDocumentStart = $selected.CompareEndpoints($end, $document, $start)
        selectionEndToDocumentEnd = $selected.CompareEndpoints($end, $document, $end)
    }
}

function Test-FullDocumentSelection($Endpoints) {
    return ($Endpoints.documentStartToEnd -lt 0 -and $Endpoints.selectionStartToEnd -lt 0 -and
        $Endpoints.selectionStartToDocumentStart -eq 0 -and $Endpoints.selectionEndToDocumentEnd -eq 0)
}

function Read-NativeSelection([string]$Phase) {
    $directory = Join-Path $runDirectory ("uia-results\" + $report.runId + '\' + $Phase)
    [void](New-Item -ItemType Directory -Path $directory -ErrorAction Stop)
    $stdout = Join-Path $directory 'stdout.jsonl'
    $stderr = Join-Path $directory 'stderr.log'
    $call = [ordered]@{ phase = $Phase; targetPid = $process.Id; maxLength = -1; directory = $directory;
        startedAtUtc = [DateTime]::UtcNow.ToString('o'); timedOut = $false; exitCode = $null; status = 'starting' }
    $report.nativeUiaCalls += $call
    Checkpoint "Native UIA ${Phase}: start supervised helper (20-second deadline)"
    $helper = Start-Process -FilePath $NativeUiaExecutable -ArgumentList @([string]$process.Id, '-1') -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    # Windows PowerShell 5 needs the process handle retained before waiting so
    # ExitCode remains available after a short-lived helper has exited.
    $helperHandle = $helper.Handle
    $call.helperPid = $helper.Id
    $call.status = 'running'
    Checkpoint "Native UIA ${Phase}: waiting for helper"
    try {
        if (-not $helper.WaitForExit(20000)) {
            $call.timedOut = $true
            $call.status = 'timed out'
            $helper.Kill()
            $call.reapedAfterTimeout = $helper.WaitForExit(5000)
            throw "Native UIA $Phase exceeded its 20-second deadline."
        }
        $call.exitCode = $helper.ExitCode
        $call.status = 'exited; validating JSONL'
    } finally {
        $helper.Dispose()
    }
    Record-Check "Native UIA $Phase exited successfully" ($call.exitCode -eq 0) "exit=$($call.exitCode); logs=$directory"
    $stages = @{}
    $lastStage = $null
    foreach ($line in Get-Content -LiteralPath $stdout) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $item = $line | ConvertFrom-Json
        if ($item.PSObject.Properties.Name -contains 'stage') {
            if ($stages.ContainsKey([string]$item.stage)) { throw "Duplicate native UIA stage: $($item.stage)" }
            if ($item.succeeded -isnot [bool]) { throw "Native UIA stage has invalid success flag: $($item.stage)" }
            $code = [Convert]::ToUInt64(([string]$item.hresult).Substring(2), 16)
            $optionalUnavailable = $item.stage -eq 'QueryInterface_IUIAutomation2' -and $item.hresult -eq '0x80004002' -and -not $item.succeeded
            if (-not $optionalUnavailable -and (-not $item.succeeded -or $code -ge 2147483648)) {
                throw "Native UIA stage failed: $($item.stage) $($item.hresult)"
            }
            $stages[[string]$item.stage] = $item
            $lastStage = [string]$item.stage
        } elseif (-not ($item.PSObject.Properties.Name -contains 'checkpoint')) {
            throw 'Native UIA stdout contains an unrecognized JSONL record.'
        }
    }
    $required = @('CoInitializeEx_MTA', 'target_window', 'CoCreateInstance_CUIAutomation', 'QueryInterface_IUIAutomation2',
        'ElementFromHandle', 'CreatePropertyCondition_native_editor', 'FindFirst_native_editor', 'editor_process_identity',
        'GetCurrentPatternAs_Value', 'ValuePattern_get_CurrentValue', 'GetCurrentPatternAs_Text', 'TextPattern_get_DocumentRange',
        'TextPattern_GetSelection', 'selection_range_count', 'selection_GetElement_0', 'document_start_to_end',
        'selection_start_to_end', 'selection_start_to_document_start', 'selection_end_to_document_end',
        'selection_GetText', 'document_GetText', 'complete')
    foreach ($stage in $required) { if (-not $stages.ContainsKey($stage)) { throw "Missing native UIA stage: $stage" } }
    if ($stages['QueryInterface_IUIAutomation2'].succeeded) {
        foreach ($stage in @('put_ConnectionTimeout_5000', 'put_TransactionTimeout_5000')) {
            if (-not $stages.ContainsKey($stage)) { throw "Missing native UIA timeout stage: $stage" }
        }
    }
    $call.optionalUia2Hresult = $stages['QueryInterface_IUIAutomation2'].hresult
    $call.stageCount = $stages.Count
    $call.value = $stages['ValuePattern_get_CurrentValue'].text
    $call.documentText = $stages['document_GetText'].text
    $call.selectionText = $stages['selection_GetText'].text
    $call.selectionCount = $stages['selection_range_count'].count
    $call.documentStartToEnd = $stages['document_start_to_end'].ordering
    $call.selectionStartToEnd = $stages['selection_start_to_end'].ordering
    $call.selectionStartToDocumentStart = $stages['selection_start_to_document_start'].ordering
    $call.selectionEndToDocumentEnd = $stages['selection_end_to_document_end'].ordering
    $report.nativeGetTextExercised = $true
    $identity = $stages['target_window'].pid -eq $process.Id -and $stages['editor_process_identity'].actualPid -eq $process.Id -and
        $stages['target_window'].hwnd -eq $process.MainWindowHandle.ToInt64()
    $complete = $lastStage -eq 'complete' -and $stages['complete'].readOnly -eq $true -and $stages['complete'].semanticAssertions -eq $false
    $exactValue = $call.value -ceq 'Pax native input 1038' -and $call.documentText -ceq 'Pax native input 1038'
    $readAll = $stages['selection_GetText'].maxLength -eq -1 -and $stages['document_GetText'].maxLength -eq -1
    Record-Check "Native UIA $Phase completed required APIs and exact document readback" ($identity -and $complete -and $exactValue -and $readAll -and $call.selectionCount -eq 1 -and $call.documentStartToEnd -lt 0) "target PID=$($process.Id); selectionCount=$($call.selectionCount); logs=$directory"
    $call.status = 'complete; semantic selection assertion pending'
    return $call
}

function Click-Point([string]$Name, [double]$X, [double]$Y, $RegionBounds) {
    Checkpoint "${Name}: read UIA geometry before mouse injection"
    $xPixel = [int][Math]::Round($X)
    $yPixel = [int][Math]::Round($Y)
    $client = Get-ProbeClientBounds
    $editorCurrent = $editor.Current
    $innerCurrent = $inner.Current
    $outerCurrent = $outer.Current
    $attempt = [ordered]@{ name = $Name; x = $xPixel; y = $yPixel; expectedRegion = (Bounds-Report $RegionBounds);
        editor = (Bounds-Report $editorCurrent.BoundingRectangle); editorOffscreen = $editorCurrent.IsOffscreen;
        innerViewport = (Bounds-Report $innerCurrent.BoundingRectangle); innerOffscreen = $innerCurrent.IsOffscreen;
        outerViewport = (Bounds-Report $outerCurrent.BoundingRectangle); outerOffscreen = $outerCurrent.IsOffscreen;
        client = (Bounds-Report $client); foreground = [ProbeWindow]::GetForegroundWindow().ToInt64() }
    $report.pointerAttempts += $attempt
    $visible = -not $editorCurrent.IsOffscreen -and -not $innerCurrent.IsOffscreen -and -not $outerCurrent.IsOffscreen
    foreach ($bounds in @($RegionBounds, $editorCurrent.BoundingRectangle, $innerCurrent.BoundingRectangle, $outerCurrent.BoundingRectangle, $client)) {
        $visible = $visible -and (Test-PointInside $bounds $xPixel $yPixel)
    }
    Record-Check "$Name point is inside visible probe viewports" $visible "physical=($xPixel,$yPixel); see pointerAttempts geometry"
    Record-Check "$Name probe is foreground" ([ProbeWindow]::GetForegroundWindow() -eq $process.MainWindowHandle) ([string]$attempt.foreground)
    $positioned = [ProbeWindow]::SetCursorPos($xPixel, $yPixel)
    $positionError = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
    $cursor = New-Object ProbeWindow+POINT
    $read = [ProbeWindow]::GetCursorPos([ref]$cursor)
    $hit = if ($read) { [ProbeWindow]::WindowFromPoint($cursor) } else { [IntPtr]::Zero }
    $hitRoot = if ($hit -ne [IntPtr]::Zero) { [ProbeWindow]::GetAncestor($hit, 2) } else { [IntPtr]::Zero }
    $attempt.cursor = @{ apiSucceeded = $positioned; lastError = $positionError; readSucceeded = $read;
        x = $cursor.X; y = $cursor.Y; hitWindow = $hit.ToInt64(); hitRoot = $hitRoot.ToInt64() }
    Record-Check "$Name cursor reached requested point" ($positioned -and $read -and $cursor.X -eq $xPixel -and $cursor.Y -eq $yPixel) "API=$positioned; actual=($($cursor.X),$($cursor.Y)); error=$positionError"
    Record-Check "$Name point is not obscured by another window" ($hitRoot -eq $process.MainWindowHandle) "hitRoot=$($hitRoot.ToInt64()); probe=$($process.MainWindowHandle.ToInt64())"
    [ProbeWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    [ProbeWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 350
}

function Save-Screenshot([string]$Name) {
    if ($null -eq $process) { return }
    $process.Refresh()
    if ($process.HasExited -or $process.MainWindowHandle -eq [IntPtr]::Zero) { return }
    $windowBounds = New-Object ProbeWindow+RECT
    if (-not [ProbeWindow]::GetWindowRect($process.MainWindowHandle, [ref]$windowBounds)) { return }
    $bounds = [System.Drawing.Rectangle]::FromLTRB($windowBounds.Left, $windowBounds.Top, $windowBounds.Right, $windowBounds.Bottom)
    $bounds.Intersect([System.Windows.Forms.SystemInformation]::VirtualScreen)
    if ($bounds.Width -le 0 -or $bounds.Height -le 0) { return }
    $report[$Name] = @{ x = $bounds.X; y = $bounds.Y; width = $bounds.Width; height = $bounds.Height }
    $bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
        $bitmap.Save((Join-Path $runDirectory $Name), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

try {
    Checkpoint 'starting checker; no qualification result yet'
    if ([System.Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0) {
        throw 'Run in the logged-in interactive desktop, not the SYSTEM session.'
    }
    Record-Check 'Checker uses physical pixel DPI coordinates' ($previousDpiContext -ne [IntPtr]::Zero) 'Per-monitor-aware V2 thread context requested'
    $existing = @(Get-Process -Name PaxCompositionProbe -ErrorAction SilentlyContinue)
    $report.preexistingProbeProcesses = @($existing | ForEach-Object { $_.Id })
    Record-Check 'No competing probe process before launch' ($existing.Count -eq 0) 'Concurrent probes would mix log evidence and may steal foreground input'
    if ($SelectionReadback -eq 'Native') {
        if ([string]::IsNullOrWhiteSpace($NativeUiaExecutable)) {
            $NativeUiaExecutable = Join-Path $PSScriptRoot "out\$Architecture\uia\native-uia.exe"
        }
        $NativeUiaExecutable = (Resolve-Path -LiteralPath $NativeUiaExecutable -ErrorAction Stop).Path
        $report.nativeUiaExecutable = $NativeUiaExecutable
        $report.nativeUiaExecutableSha256 = (Get-FileHash -LiteralPath $NativeUiaExecutable -Algorithm SHA256).Hash
        Checkpoint 'Native UIA helper resolved; no managed text-range readback will be used'
    }
    $start = @{ FilePath = Join-Path $runDirectory 'PaxCompositionProbe.exe'; WorkingDirectory = $runDirectory; PassThru = $true }
    $arguments = @()
    if ($Producer -eq 'dx12') { $arguments += '--dx12' }
    if ($Producer -eq 'pax') { $arguments += '--pax' }
    if ($AllowWarp) { $arguments += '--allow-warp' }
    if ($arguments.Count -gt 0) { $start.ArgumentList = $arguments }
    $process = Start-Process @start
    $report.process = $process.Id
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
        if ($process.HasExited) { throw "Probe exited before window appeared: $($process.ExitCode)" }
    } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Probe window did not appear.' }
    Move-ProbeWindow 1180 880 20
    $foregroundRequested = [ProbeWindow]::SetForegroundWindow($process.MainWindowHandle)
    $report.foregroundRequestSucceeded = $foregroundRequested
    $frameDeadline = [DateTime]::UtcNow.AddSeconds(60)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        if ($process.HasExited) { throw 'Probe exited during producer initialization.' }
        $runLog = Read-RunLog
    } while (-not $runLog.Contains("first_frame_presented producer=$Producer") -and [DateTime]::UtcNow -lt $frameDeadline)
    Checkpoint 'UIA AutomationElement.FromHandle'
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $editor = Find-Control $root 'native-editor'
    $inner = Find-Control $root 'inner-scroll'
    $outer = Find-Control $root 'outer-scroll'
    Record-Check 'Native UIA text control' ($editor.Current.ControlType -eq [System.Windows.Automation.ControlType]::Edit) $editor.Current.ClassName
    Record-Check 'GPU frame published' ((Read-RunLog).Contains("first_frame_presented producer=$Producer")) $Producer
    $pause = Find-Control $root 'pause-animation'
    Checkpoint 'UIA pause.SetFocus before pointer ownership tests'
    $pause.SetFocus()
    Record-Check 'Pointer test begins with editor unfocused' ($pause.Current.HasKeyboardFocus -and -not $editor.Current.HasKeyboardFocus) 'Pause button has focus; animation has not been paused'
    $editorBounds = $editor.Current.BoundingRectangle
    $scale = [ProbeWindow]::GetDpiForWindow($process.MainWindowHandle) / 96.0
    $report.dpiScale = $scale
    Record-Check 'Editor geometry and DPI are available' ((Test-ValidBounds $editorBounds) -and $scale -gt 0) "scale=$scale; offscreen=$($editor.Current.IsOffscreen)"
    $clearCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty, 'clear-mask-input')
    Checkpoint 'UIA optional clear-mask-input geometry lookup'
    $clear = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $clearCondition)
    if ($null -ne $clear -and (Test-ValidBounds $clear.Current.BoundingRectangle)) {
        $clearBounds = $clear.Current.BoundingRectangle
        $clearX = $clearBounds.Left + $clearBounds.Width / 2
        $clearY = $clearBounds.Top + $clearBounds.Height / 2
        $report.clearPointSource = 'Center of clear-mask-input UIA bounds; the affine rectangle center remains inside its transformed shape.'
        Record-Check 'Clear mask UIA region is visible' (-not $clear.Current.IsOffscreen) 'clear-mask-input exposes valid bounds'
    } else {
        $clearBounds = $editorBounds
        $clearX = $editorBounds.Left + 16 * $scale
        $clearY = $editorBounds.Top + $editorBounds.Height / 2
        $report.clearPointSource = 'clear-mask-input has no usable UIA peer. Fallback is specific to main.cpp: 48-DIP clear strip, 420x52 editor, rotation -3 degrees and skewX 4 degrees; 16 DIP from the editor AABB left at its vertical midpoint lies inside that strip. Viewport and native HWND checks still apply.'
    }
    Click-Point 'Clear mask' $clearX $clearY $clearBounds
    Record-Check 'Transparent mask pointer belongs to Pax' ((Read-RunLog).Contains('pax_clear_mask_pointer_pressed') -and -not $editor.Current.HasKeyboardFocus) 'Real mouse press; native editor remained unfocused'
    Click-Point 'Visible mask' ($editorBounds.Left + 180 * $scale) ($editorBounds.Top + $editorBounds.Height / 2) $editorBounds
    Record-Check 'Visible mask pointer belongs to native editor' ((Read-RunLog).Contains('native_pointer_pressed') -and $editor.Current.HasKeyboardFocus) 'Real mouse press; native editor gained focus'
    Checkpoint 'UIA editor.SetFocus before keyboard editing'
    $editor.SetFocus()
    [System.Windows.Forms.SendKeys]::SendWait('^a')
    [System.Windows.Forms.SendKeys]::SendWait('Pax native input 1038')
    Start-Sleep -Milliseconds 800
    Checkpoint 'UIA editor.GetCurrentPattern(ValuePattern) and Current.Value'
    $value = $editor.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    Record-Check 'Keyboard editing through native mask' ($value.Current.Value -eq 'Pax native input 1038') $value.Current.Value
    if ($SelectionReadback -eq 'Endpoints') {
        Checkpoint 'UIA Endpoints mode: GetCurrentPattern(TextPattern); GetText will not be called'
        $text = $editor.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
        [System.Windows.Forms.SendKeys]::SendWait('^{HOME}')
        Start-Sleep -Milliseconds 200
        $collapsed = Read-SelectionEndpoints $text 'collapsed negative control'
        $report.selectionEndpoints = [ordered]@{ collapsedNegativeControl = $collapsed }
        Record-Check 'Endpoint document contains nonempty text' ($collapsed.documentStartToEnd -lt 0) ($collapsed | ConvertTo-Json -Compress)
        Record-Check 'Endpoint negative control is a caret at document start' ($collapsed.selectionStartToEnd -eq 0 -and
            $collapsed.selectionStartToDocumentStart -eq 0 -and $collapsed.selectionEndToDocumentStart -eq 0 -and
            -not (Test-FullDocumentSelection $collapsed)) ($collapsed | ConvertTo-Json -Compress)
    } elseif ($SelectionReadback -eq 'Native') {
        [System.Windows.Forms.SendKeys]::SendWait('^{HOME}')
        Start-Sleep -Milliseconds 200
        $collapsed = Read-NativeSelection 'collapsed-negative-control'
        Record-Check 'Native UIA negative control is an empty caret at document start' ($collapsed.selectionText -ceq '' -and
            $collapsed.selectionStartToEnd -eq 0 -and $collapsed.selectionStartToDocumentStart -eq 0 -and
            $collapsed.selectionEndToDocumentEnd -lt 0) 'Empty selected GetText; full document GetText and ValuePattern remain exact'
        $collapsed.status = 'passed collapsed-caret negative control'
    }
    [System.Windows.Forms.SendKeys]::SendWait('^a')
    if ($SelectionReadback -eq 'Endpoints') {
        Start-Sleep -Milliseconds 200
        $selectedBefore = Read-SelectionEndpoints $text 'before animation'
        $report.selectionEndpoints.beforeAnimation = $selectedBefore
        Record-Check 'Endpoint selection spans the full document before animation' (Test-FullDocumentSelection $selectedBefore) ($selectedBefore | ConvertTo-Json -Compress)
    } elseif ($SelectionReadback -eq 'Native') {
        Start-Sleep -Milliseconds 200
        $selectedBefore = Read-NativeSelection 'before-animation'
        Record-Check 'Native UIA selection text and endpoints span the full document before animation' ((Test-FullDocumentSelection $selectedBefore) -and $selectedBefore.selectionText -ceq 'Pax native input 1038') 'Native selected GetText and full-document endpoints match'
        $selectedBefore.status = 'passed full-selection control'
    }
    $frameBeforeSelectionWait = Read-NativeFrameCheckpoint
    Start-Sleep -Seconds 2
    $frameAfterSelectionWait = Read-NativeFrameCheckpoint
    $report.selectionFrameProgress = @{ before = $frameBeforeSelectionWait; after = $frameAfterSelectionWait }
    Record-Check 'Native rendering advances while selection is retained' ($frameAfterSelectionWait.frame -gt $frameBeforeSelectionWait.frame) "native frame=$($frameBeforeSelectionWait.frame) -> $($frameAfterSelectionWait.frame) during the two-second selection interval"
    Checkpoint 'Capture selected native editor before UIA text-range calls'
    Save-Screenshot 'selected-before-uia.png'
    if ($SelectionReadback -eq 'GetText') {
        Checkpoint 'UIA editor.GetCurrentPattern(TextPattern)'
        $text = $editor.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
        Checkpoint 'UIA TextPattern.GetSelection'
        $ranges = $text.GetSelection()
        Record-Check 'Native selection exposes a text range' ($ranges.Count -gt 0) "ranges=$($ranges.Count)"
        $report.textRangeGetTextExercised = $true
        $report.textRangeGetTextStatus = 'GetText(-1) attempted; completion and exact selected-text match pending'
        Checkpoint 'UIA TextPatternRange.GetText(-1); a native UIAutomationCore crash here leaves this checkpoint incomplete'
        $selection = $ranges[0].GetText(-1).TrimEnd("`r", "`n")
        Record-Check 'Selection survives animated repaint' ($selection -eq 'Pax native input 1038') $selection
        $report.textRangeGetTextQualified = $true
        $report.textRangeGetTextStatus = 'GetText(-1) returned the expected selected text'
    } elseif ($SelectionReadback -eq 'Endpoints') {
        $selectedAfter = Read-SelectionEndpoints $text 'after animation'
        $report.selectionEndpoints.afterAnimation = $selectedAfter
        Record-Check 'Endpoint selection survives animated repaint' (Test-FullDocumentSelection $selectedAfter) ($selectedAfter | ConvertTo-Json -Compress)
    } else {
        $selectedAfter = Read-NativeSelection 'after-animation'
        Record-Check 'Native UIA selection text and endpoints survive animated repaint' ((Test-FullDocumentSelection $selectedAfter) -and $selectedAfter.selectionText -ceq 'Pax native input 1038') 'Native selected GetText and full-document endpoints remain exact after frame advancement'
        $selectedAfter.status = 'passed retained full-selection control'
        $report.nativeGetTextQualified = $true
    }
    Save-Screenshot 'selected-masked.png'

    Checkpoint 'UIA inner ScrollPattern and horizontal SetScrollPercent'
    $scroll = $inner.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
    $scroll.SetScrollPercent(40, [System.Windows.Automation.ScrollPattern]::NoScroll)
    Start-Sleep -Milliseconds 600
    Record-Check 'Native horizontal scrolling' ($scroll.Current.HorizontalScrollPercent -gt 1) ([string]$scroll.Current.HorizontalScrollPercent)
    Record-Check 'Text retained through scroll' ($value.Current.Value -eq 'Pax native input 1038') $value.Current.Value
    $scroll.SetScrollPercent(0, [System.Windows.Automation.ScrollPattern]::NoScroll)
    Checkpoint 'UIA outer ScrollPattern and vertical SetScrollPercent'
    $vertical = $outer.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
    $vertical.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll, 35)
    Start-Sleep -Milliseconds 500
    Record-Check 'Native vertical scrolling' ($vertical.Current.VerticalScrollPercent -gt 1) ([string]$vertical.Current.VerticalScrollPercent)
    $vertical.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll, 0)
    Move-ProbeWindow 1120 840 30
    Start-Sleep -Seconds 1
    Record-Check 'Text retained through window resize' ($value.Current.Value -eq 'Pax native input 1038') $value.Current.Value
    Save-Screenshot 'resized-masked.png'
    Record-Check 'Rendering remained healthy' (-not (Read-RunLog).Contains('render failed:')) 'No rendering exception in this run'
    $scriptedChecksPassed = $true
} catch {
    $report.result = 'failed'
    $report.error = $_.Exception.Message
    $report.runLog = Read-RunLog
    if (Test-Path $runDirectory) { Save-Screenshot 'failure.png' }
} finally {
    try {
        if ($null -ne $process -and -not $KeepOpen) {
            $process.Refresh()
            if (-not $process.HasExited) { [void]$process.CloseMainWindow() }
            $exited = $process.WaitForExit(15000)
            $report.exitCode = if ($exited) { $process.ExitCode } else { $null }
            Record-Check 'Clean process exit' ($exited -and $process.ExitCode -eq 0) "exited=$exited code=$($report.exitCode)"
            Record-Check 'Application loop shut down' ((Read-RunLog).Contains('Application.Start returned cleanly')) 'XAML application loop returned normally'
            if ($Producer -eq 'pax') {
                Record-Check 'Pax exports released after native copy fence' ((Read-RunLog).Contains('pax_wgpu context dropped after native copy fence')) 'Rust context destruction observed'
            }
        } elseif ($KeepOpen) {
            $report.lifecycle = 'not qualified because KeepOpen leaves the process running'
        }
    } catch {
        $report.result = 'failed'
        $report.shutdownError = $_.Exception.Message
        $report.runLog = Read-RunLog
    }
    if ($scriptedChecksPassed -and $report.result -ne 'failed') {
        $report.result = if ($SelectionReadback -eq 'Endpoints') {
            'passed endpoint-selection and remaining scripted checks; UIA GetText not exercised/not qualified; visual artifacts require inspection; IME unverified'
        } elseif ($SelectionReadback -eq 'Native') {
            'passed native COM UIA GetText/selection and remaining scripted checks; managed GetText not exercised/not qualified; visual artifacts require inspection; IME unverified'
        } else {
            'passed scripted checks; visual artifacts require inspection; IME unverified'
        }
    }
    $report.finishedAtUtc = [DateTime]::UtcNow.ToString('o')
    Checkpoint 'checker finished'
}
if ($report.result -eq 'failed') { throw "Interactive qualification failed; see interactive-result.json" }
