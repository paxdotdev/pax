param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'ARM64',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [ValidateRange(1, 300)][int]$TimeoutSeconds = 60,
    [switch]$AllowSoftwareDiagnostic,
    [string]$OutputRoot = (Join-Path $PSScriptRoot 'out\capabilities')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Every invocation owns a new directory. Crashes cannot reuse a previous pass,
# and the runner never removes another probe's evidence or an active run's files.
$runId = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ') + '-' + [Guid]::NewGuid().ToString('N')
$outputDirectory = [IO.Path]::GetFullPath($OutputRoot)
New-Item -ItemType Directory -Force $outputDirectory | Out-Null
$runDirectory = Join-Path $outputDirectory $runId
New-Item -ItemType Directory $runDirectory | Out-Null
$stdoutPath = Join-Path $runDirectory 'events.jsonl'
$stderrPath = Join-Path $runDirectory 'stderr.log'
$reportPath = Join-Path $runDirectory 'preflight.json'
$executable = Join-Path $PSScriptRoot "out\$Architecture\$Configuration\pax-wgpu-preflight.exe"
$report = [ordered]@{
    schemaVersion = 1
    runId = $runId
    startedAt = [DateTime]::UtcNow.ToString('o')
    architecture = $Architecture
    configuration = $Configuration
    operatingSystem = [Environment]::OSVersion.VersionString
    processArchitecture = $env:PROCESSOR_ARCHITECTURE
    sessionId = [Diagnostics.Process]::GetCurrentProcess().SessionId
    mode = $(if ($AllowSoftwareDiagnostic) { 'hardware-or-software-diagnostic' } else { 'hardware-only' })
    timeoutSeconds = $TimeoutSeconds
    executable = $executable
    executableSha256 = $null
    status = 'failed'
    passed = $false
    hardwarePassed = $false
    softwareDiagnosticPassed = $false
    nativeCompositionQualified = $false
    physicalPerformanceQualified = $false
    processExitCode = $null
    timedOut = $false
    events = @()
    producerResult = $null
    error = $null
}
$process = $null
try {
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw 'Build the matching Rust producer and preflight executable with build-wgpu.ps1 first.'
    }
    $report.executableSha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash
    $start = @{
        FilePath = $executable
        WorkingDirectory = $runDirectory
        RedirectStandardOutput = $stdoutPath
        RedirectStandardError = $stderrPath
        PassThru = $true
        NoNewWindow = $true
    }
    if ($AllowSoftwareDiagnostic) { $start.ArgumentList = '--allow-software-diagnostic' }
    $process = Start-Process @start
    # Windows PowerShell 5 can return a Process without a retained native handle.
    # Open/cache it while the child is alive so ExitCode remains available after
    # WaitForExit; the Process owns this handle until Dispose in finally.
    $processHandle = $process.Handle
    if ($processHandle -eq [IntPtr]::Zero) { throw 'Could not retain the preflight process handle.' }
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $report.timedOut = $true
        $report.status = 'timeout'
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        if (-not $process.WaitForExit(5000)) { throw 'Preflight timed out and did not terminate after forced stop.' }
    }
    $process.Refresh()
    $report.processExitCode = $process.ExitCode
    if ($null -eq $report.processExitCode) { throw 'Preflight exited without an available process exit code.' }
    if (Test-Path -LiteralPath $stdoutPath) {
        foreach ($line in Get-Content -LiteralPath $stdoutPath) {
            if (-not [string]::IsNullOrWhiteSpace($line)) {
                $report.events += ($line | ConvertFrom-Json)
            }
        }
    }
    if ($report.timedOut) { throw "Preflight exceeded its $TimeoutSeconds second deadline; partial events are retained." }
    $results = @($report.events | Where-Object { $_.event -eq 'result' })
    if ($results.Count -ne 1) {
        $report.status = 'incomplete'
        throw "Expected one final result, received $($results.Count); exit code $($report.processExitCode)."
    }
    $result = $results[0]
    $report.producerResult = $result
    if ($result.schemaVersion -ne 1 -or $result.mode -ne $report.mode) {
        throw 'Preflight result schema or requested mode does not match this invocation.'
    }
    # Treat only an expected, successful process exit plus a complete result as
    # positive evidence. In particular, a final log line before a crash is no pass.
    $succeeded = ($report.processExitCode -eq 0 -and $result.passed -eq $true)
    $report.hardwarePassed = ($succeeded -and $result.hardwarePassed -eq $true)
    $report.softwareDiagnosticPassed = ($succeeded -and [bool]$AllowSoftwareDiagnostic -and $result.softwareDiagnosticPassed -eq $true)
    $report.passed = ($report.hardwarePassed -or $report.softwareDiagnosticPassed)
    if (-not $report.passed) { throw "Pax/WGPU capability preflight failed: $($result.reason)" }
    $report.status = $(if ($report.hardwarePassed) { 'hardware-available' } else { 'software-diagnostic-only' })
} catch {
    $report.error = $_.Exception.Message
} finally {
    if ($null -ne $process) {
        try {
            $process.Refresh()
            if (-not $process.HasExited) {
                Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
                if (-not $process.WaitForExit(5000)) { throw 'Preflight process did not terminate during cleanup.' }
            }
        } catch {
            $report.passed = $false
            $report.hardwarePassed = $false
            $report.softwareDiagnosticPassed = $false
            $report.status = 'cleanup-failed'
            $report.error = "$($report.error) Cleanup: $($_.Exception.Message)"
        } finally {
            $process.Dispose()
        }
    }
    $report['finishedAt'] = [DateTime]::UtcNow.ToString('o')
    $report | ConvertTo-Json -Depth 15 | Set-Content -LiteralPath $reportPath -Encoding UTF8
    Write-Output $reportPath
    Write-Output "status=$($report.status) hardwarePassed=$($report.hardwarePassed) softwareDiagnosticPassed=$($report.softwareDiagnosticPassed)"
}
if (-not $report.passed) {
    Write-Error $report.error -ErrorAction Continue
    exit 1
}
