param(
    [Parameter(Mandatory=$true)][string]$Cli,
    [Parameter(Mandatory=$true)][string]$Project,
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
$Cli=(Resolve-Path -LiteralPath $Cli).Path
$Project=(Resolve-Path -LiteralPath $Project).Path
$template=Join-Path $Project 'src\lib.pax'
$original=[IO.File]::ReadAllBytes($template)
$text=[Text.Encoding]::UTF8.GetString($original)
$label='Outer native scroller';$replacement='Live Windows reload verified'
if(([regex]::Matches($text,[regex]::Escape($label))).Count -ne 1){throw 'Expected exactly one fixture reload label'}
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ([guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $run)
$result=[ordered]@{passed=$false;checks=@();appPid=$null;sourceSha256=(Get-FileHash $template -Algorithm SHA256).Hash;restoredSha256=$null;error=$null}
$cliProcess=$null;$application=$null;$oldLog=$env:PAX_WINDOWS_HOST_LOG
$env:PAX_WINDOWS_HOST_LOG=Join-Path $run 'native-host.log'
function Check([string]$Name,[bool]$Passed){
    $result.checks+=@{name=$Name;passed=$Passed}
    if(-not $Passed){throw $Name}
}
function Owned-Application($Candidate){
    if(-not $Candidate.Path.StartsWith($Project.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)){return $false}
    $ancestor=[uint32]$Candidate.Id
    for($depth=0;$depth -lt 12;$depth++){
        $processInfo=Get-CimInstance Win32_Process -Filter "ProcessId=$ancestor"
        if($null -eq $processInfo){return $false}
        $ancestor=[uint32]$processInfo.ParentProcessId
        if($ancestor -eq $cliProcess.Id){return $true}
        if($ancestor -eq 0){return $false}
    }
    return $false
}
function Wait-Label([string]$Name,[int]$TimeoutSeconds){
    $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do{
        if($cliProcess.HasExited){throw "CLI exited before reload completed: $($cliProcess.ExitCode)"}
        if($null -eq $script:application){
            $candidates=@(Get-Process native-desktop-smoke -ErrorAction SilentlyContinue|Where-Object{$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.MainWindowHandle -ne [IntPtr]::Zero})
            if($candidates.Count -gt 1){throw 'Multiple candidate fixture processes'}
            if($candidates.Count -eq 1){
                if(-not (Owned-Application $candidates[0])){throw 'Fixture process is not owned by this CLI launch'}
                $script:application=$candidates[0];$script:applicationHandle=$script:application.Handle;$result.appPid=$script:application.Id
            }
        }
        if($null -ne $script:application){
            $script:application.Refresh()
            if($script:application.HasExited){throw 'Native application exited during template reload'}
            $root=[Windows.Automation.AutomationElement]::FromHandle($script:application.MainWindowHandle)
            $condition=New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::NameProperty,$Name)
            if($null -ne $root.FindFirst([Windows.Automation.TreeScope]::Descendants,$condition)){return}
        }
        Start-Sleep -Milliseconds 100
    }while([DateTime]::UtcNow -lt $deadline)
    throw "Native template reload timeout: $Name"
}
try{
    Check 'Interactive desktop session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0)
    Check 'No competing fixture process' (@(Get-Process native-desktop-smoke -ErrorAction SilentlyContinue).Count -eq 0)
    $cliProcess=Start-Process -FilePath $Cli -ArgumentList @('run','--path',('"'+$Project+'"'),'--target','windows','--hot-reload=pax') -WorkingDirectory $Project -PassThru -RedirectStandardOutput "$run\cli.stdout.log" -RedirectStandardError "$run\cli.stderr.log"
    $cliHandle=$cliProcess.Handle
    Wait-Label $label 600
    $originalPid=$application.Id;$originalWindow=$application.MainWindowHandle
    Check 'CLI launched a separate native application with its design server' ($originalPid -ne $cliProcess.Id)
    [IO.File]::WriteAllText($template,$text.Replace($label,$replacement),(New-Object Text.UTF8Encoding($false)))
    Wait-Label $replacement 45
    Check 'Saved template updates native text in the same process and window' ($application.Id -eq $originalPid -and $application.MainWindowHandle -eq $originalWindow)
    [IO.File]::WriteAllBytes($template,$original)
    Wait-Label $label 45
    Check 'Exact source restoration updates the same native application' ($application.Id -eq $originalPid -and $application.MainWindowHandle -eq $originalWindow)
    [void]$application.CloseMainWindow()
    Check 'Native application closes cleanly after reload' ($application.WaitForExit(30000) -and $application.ExitCode -eq 0)
    Check 'CLI reaps its development server after native close' ($cliProcess.WaitForExit(30000) -and $cliProcess.ExitCode -eq 0)
    $result.passed=$true
}catch{$result.error=$_.Exception.Message}finally{
    try{
        [IO.File]::WriteAllBytes($template,$original)
        $result.restoredSha256=(Get-FileHash $template -Algorithm SHA256).Hash
        if($result.restoredSha256 -ne $result.sourceSha256){throw 'Source restoration hash mismatch'}
    }catch{$result.passed=$false;$result.restorationError=$_.Exception.Message}
    if($application){
        if(-not $application.HasExited){[void]$application.CloseMainWindow();if(-not $application.WaitForExit(25000)){$application.Kill();$result.passed=$false;$result.error='Application required forced cleanup'}}
        $application.Dispose()
    }
    if($cliProcess){
        if(-not $cliProcess.HasExited){
            $result.passed=$false;$result.forcedCliCleanup=$true
            & "$env:SystemRoot\System32\taskkill.exe" /PID $cliProcess.Id /T /F|Out-Null
            [void]$cliProcess.WaitForExit(5000)
        }
        $cliProcess.Dispose()
    }
    $env:PAX_WINDOWS_HOST_LOG=$oldLog
    $result|ConvertTo-Json -Depth 8|Set-Content -LiteralPath "$run\result.json" -Encoding UTF8
    Write-Output "$run\result.json"
}
if(-not $result.passed){exit 1}
