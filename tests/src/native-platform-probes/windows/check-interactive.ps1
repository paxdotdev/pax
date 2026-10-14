param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'ARM64',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [ValidateSet('d3d11', 'dx12', 'pax')][string]$Producer = 'd3d11',
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
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT bounds);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extraInfo);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hwnd, int x, int y, int width, int height, bool repaint);
}
'@
# Use physical pixels for both MoveWindow and screenshot bounds at mixed DPI.
[void][ProbeWindow]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$runDirectory = Join-Path $PSScriptRoot "out\$Architecture\$Configuration"
$report = [ordered]@{
    architecture = $Architecture
    configuration = $Configuration
    producer = $Producer
    allowWarp = [bool]$AllowWarp
    process = $null
    checks = @()
    ime = 'not tested; keyboard text injection does not establish IME composition support'
    gpu = 'see composition-probe.log adapter and device type; VM evidence only'
}
$process = $null
$logPath = Join-Path $runDirectory 'composition-probe.log'
$logOffset = if (Test-Path $logPath) { (Get-Item $logPath).Length } else { 0 }

function Record-Check([string]$Name, [bool]$Passed, [string]$Detail) {
    $report.checks += [ordered]@{ name = $Name; passed = $Passed; detail = $Detail }
    if (-not $Passed) { throw "$Name failed: $Detail" }
}

function Find-Control($Root, [string]$AutomationId) {
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty, $AutomationId)
    $result = $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
    if ($null -eq $result) { throw "Control not found: $AutomationId" }
    return $result
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

function Click-Point([double]$X, [double]$Y) {
    [void][ProbeWindow]::SetCursorPos([int]$X, [int]$Y)
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
    $bounds.Intersect([System.Windows.Forms.Screen]::PrimaryScreen.Bounds)
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
    if ([System.Diagnostics.Process]::GetCurrentProcess().SessionId -eq 0) {
        throw 'Run in the logged-in interactive desktop, not the SYSTEM session.'
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
    [void][ProbeWindow]::MoveWindow($process.MainWindowHandle, 20, 20, 1180, 880, $true)
    [void][ProbeWindow]::SetForegroundWindow($process.MainWindowHandle)
    $frameDeadline = [DateTime]::UtcNow.AddSeconds(60)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        if ($process.HasExited) { throw 'Probe exited during producer initialization.' }
        $runLog = Read-RunLog
    } while (-not $runLog.Contains("first_frame_presented producer=$Producer") -and [DateTime]::UtcNow -lt $frameDeadline)
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
    $editor = Find-Control $root 'native-editor'
    Record-Check 'Native UIA text control' ($editor.Current.ControlType -eq [System.Windows.Automation.ControlType]::Edit) $editor.Current.ClassName
    Record-Check 'GPU frame published' ((Read-RunLog).Contains("first_frame_presented producer=$Producer")) $Producer
    $pause = Find-Control $root 'pause-animation'
    $pause.SetFocus()
    $editorBounds = $editor.Current.BoundingRectangle
    $scale = [ProbeWindow]::GetDpiForWindow($process.MainWindowHandle) / 96.0
    $report.dpiScale = $scale
    # The clear strip and editor share the affine parent. These points stay well
    # inside each region despite its small rotation and skew.
    Click-Point ($editorBounds.Left + 16 * $scale) ($editorBounds.Top + $editorBounds.Height / 2)
    Record-Check 'Transparent mask pointer belongs to Pax' ((Read-RunLog).Contains('pax_clear_mask_pointer_pressed') -and -not $editor.Current.HasKeyboardFocus) 'Real mouse press; native editor remained unfocused'
    Click-Point ($editorBounds.Left + 180 * $scale) ($editorBounds.Top + $editorBounds.Height / 2)
    Record-Check 'Visible mask pointer belongs to native editor' ((Read-RunLog).Contains('native_pointer_pressed') -and $editor.Current.HasKeyboardFocus) 'Real mouse press; native editor gained focus'
    $editor.SetFocus()
    [System.Windows.Forms.SendKeys]::SendWait('^a')
    [System.Windows.Forms.SendKeys]::SendWait('Pax native input 1038')
    Start-Sleep -Milliseconds 800
    $value = $editor.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    Record-Check 'Keyboard editing through native mask' ($value.Current.Value -eq 'Pax native input 1038') $value.Current.Value
    [System.Windows.Forms.SendKeys]::SendWait('^a')
    Start-Sleep -Seconds 2
    $text = $editor.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
    $selection = $text.GetSelection()[0].GetText(-1).TrimEnd("`r", "`n")
    Record-Check 'Selection survives animated repaint' ($selection -eq 'Pax native input 1038') $selection
    Save-Screenshot 'selected-masked.png'

    $inner = Find-Control $root 'inner-scroll'
    $scroll = $inner.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
    $scroll.SetScrollPercent(40, [System.Windows.Automation.ScrollPattern]::NoScroll)
    Start-Sleep -Milliseconds 600
    Record-Check 'Native horizontal scrolling' ($scroll.Current.HorizontalScrollPercent -gt 1) ([string]$scroll.Current.HorizontalScrollPercent)
    Record-Check 'Text retained through scroll' ($value.Current.Value -eq 'Pax native input 1038') $value.Current.Value
    $scroll.SetScrollPercent(0, [System.Windows.Automation.ScrollPattern]::NoScroll)
    $outer = Find-Control $root 'outer-scroll'
    $vertical = $outer.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
    $vertical.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll, 35)
    Start-Sleep -Milliseconds 500
    Record-Check 'Native vertical scrolling' ($vertical.Current.VerticalScrollPercent -gt 1) ([string]$vertical.Current.VerticalScrollPercent)
    $vertical.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll, 0)
    [void][ProbeWindow]::MoveWindow($process.MainWindowHandle, 30, 30, 1120, 840, $true)
    Start-Sleep -Seconds 1
    Record-Check 'Text retained through window resize' ($value.Current.Value -eq 'Pax native input 1038') $value.Current.Value
    Save-Screenshot 'resized-masked.png'
    Record-Check 'Rendering remained healthy' (-not (Read-RunLog).Contains('render failed:')) 'No rendering exception in this run'
    $report.result = 'passed scripted checks; visual artifacts require inspection; IME unverified'
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
    $report | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $PSScriptRoot 'interactive-result.json') -Encoding UTF8
}
if ($report.result -eq 'failed') { throw "Interactive qualification failed; see interactive-result.json" }
