param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-wheel.ps1'),[string]$RetainedResult='')
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $Runner).Path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
foreach($name in @('Test-WheelReadiness','Get-WheelSnapshotLines','Close-WheelProcess','Write-ProbeReport','Read-ProbeReport')){
 $f=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name},$true))
 if($f.Count -ne 1){throw ('Expected one helper: '+$name)};. ([scriptblock]::Create($f[0].Extent.Text))
}
$checks=@();function Assert([string]$name,[bool]$ok){$script:checks+=@{name=$name;passed=$ok};if(!$ok){throw $name}}
Assert 'Visible owned WinUI plus first publication is ready' (Test-WheelReadiness 123 'WinUIDesktopWin32WindowClass' $true 81 81 $true)
Assert 'Console with publication is not ready' (!(Test-WheelReadiness 123 'ConsoleWindowClass' $true 81 81 $true))
Assert 'Invisible WinUI is not ready' (!(Test-WheelReadiness 123 'WinUIDesktopWin32WindowClass' $false 81 81 $true))
Assert 'Foreign PID is not ready' (!(Test-WheelReadiness 123 'WinUIDesktopWin32WindowClass' $true 82 81 $true))
Assert 'No publication is not ready' (!(Test-WheelReadiness 123 'WinUIDesktopWin32WindowClass' $true 81 81 $false))
Assert 'Null window is not ready' (!(Test-WheelReadiness 0 'WinUIDesktopWin32WindowClass' $true 81 81 $true))
Assert 'Different class case is not ready' (!(Test-WheelReadiness 123 'winuidesktopwin32windowclass' $true 81 81 $true))
Assert 'Missing PID is not ready' (!(Test-WheelReadiness 123 'WinUIDesktopWin32WindowClass' $true 0 0 $true))
Assert 'Empty log has no snapshots' (@(Get-WheelSnapshotLines '').Count -eq 0)
Assert 'Partial first line cannot advance snapshot' (@(Get-WheelSnapshotLines 'partial').Count -eq 0)
$lines=@(Get-WheelSnapshotLines "first`npartial");Assert 'Partial tail does not hide complete line' ($lines.Count -eq 1 -and $lines[0] -ceq 'first')
$lines=@(Get-WheelSnapshotLines "first`r`nsecond`r`n");Assert 'CRLF preserves two exact lines' ($lines.Count -eq 2 -and $lines[1] -ceq 'second')
$lines=@(Get-WheelSnapshotLines "first`nsecond`n");Assert 'LF preserves two exact lines' ($lines.Count -eq 2 -and $lines[1] -ceq 'second')
$lines=@(Get-WheelSnapshotLines ("state="+[char]0x03b1+"`n"));Assert 'Snapshot remains a plain Unicode string' ($lines.Count -eq 1 -and $lines[0] -is [string] -and $lines[0] -ceq ('state='+[char]0x03b1))
function Fake-Process([bool]$exited,[bool]$closes){
 $p=[pscustomobject]@{HasExited=$exited;ExitCode=0;closes=$closes;closeCalls=0;killCalls=0;waits=@()}
 $p|Add-Member ScriptMethod CloseMainWindow {$this.closeCalls++;return $true}
 $p|Add-Member ScriptMethod WaitForExit {param($ms)$this.waits+=@($ms);if($this.closes -or $this.killCalls){$this.HasExited=$true};return $this.HasExited}
 $p|Add-Member ScriptMethod Kill {$this.killCalls++;$this.ExitCode=-1}
 return $p
}
$report=[ordered]@{result='failed';error='startup before readiness'};Close-WheelProcess $null
Assert 'Failure before process creation invents no close receipt' (!$report.Contains('close') -and $report.error -ceq 'startup before readiness')
$report=[ordered]@{result='failed';error='startup before readiness'};$p=Fake-Process $false $true;Close-WheelProcess $p
Assert 'Unready owned process closes normally without input' ($p.closeCalls -eq 1 -and $p.killCalls -eq 0 -and $report.close.graceful -and $report.error -ceq 'startup before readiness')
$report=[ordered]@{result='passed'};$p=Fake-Process $true $true;Close-WheelProcess $p
Assert 'Already closed process is not closed again' ($p.closeCalls -eq 0 -and $p.killCalls -eq 0 -and $report.close.graceful -and $report.close.exitCode -eq 0)
$report=[ordered]@{result='failed';error='original input'};$p=Fake-Process $false $false;Close-WheelProcess $p
Assert 'Owned close timeout remains forced and bounded' ($p.closeCalls -eq 1 -and $p.killCalls -eq 1 -and ($p.waits -join ',') -ceq '15000,5000' -and !$report.close.graceful -and $report.error -ceq 'original input')
if($RetainedResult){
$retained=[IO.File]::ReadAllText((Resolve-Path $RetainedResult).Path)|ConvertFrom-Json
Assert 'Historical oracle remains 121 checks and 14 cases' ($retained.result -eq 'pass' -and $retained.checks.Count -eq 121 -and $retained.cases.Count -eq 14)
function Test-OwnedPeers($peers){
 $after=@{native=@($peers)}
  $nativeKinds=@($after.native|ForEach-Object{$_.kind}|Sort-Object)
  $nativeIds=@($after.native|ForEach-Object{$_.id}|Sort-Object -Unique)
  $ownedPeers=@($after.native|Where-Object{$_.id -cmatch '^pax-node-[0-9]+$' -and $_.class -ceq 'ScrollViewer'})
 return ($after.native.Count -eq 2 -and ($nativeKinds -join ',') -ceq 'inner,outer' -and $nativeIds.Count -eq 2 -and $ownedPeers.Count -eq 2)
}
function Test-Cancellation($after,[bool]$cancelled){if($cancelled){return ($after.prevented -and $after.raw.Contains(' prevented=true]'))}else{return (!$after.prevented -and $after.raw.Contains(' prevented=false]'))}}
foreach($case in $retained.cases){
 Assert ($case.name+': retained native peers') (Test-OwnedPeers $case.after.native)
 Assert ($case.name+': retained event-time cancellation') (Test-Cancellation $case.after ($case.name -in @('cancel-inner','cancel-painted-stroke','cancel-outer')))
}
foreach($mode in @('duplicate-kind','duplicate-id','empty-id','wrong-class')){
 $peers=($retained.cases[0].after.native|ConvertTo-Json -Depth 6|ConvertFrom-Json)
 switch($mode){'duplicate-kind'{$peers[1].kind=$peers[0].kind};'duplicate-id'{$peers[1].id=$peers[0].id};'empty-id'{$peers[1].id=''};'wrong-class'{$peers[1].class='Other'}}
 Assert ($mode+' rejected') (!(Test-OwnedPeers $peers))
}
foreach($case in @($retained.cases|Where-Object{$_.name -in @('cancel-inner','cancel-painted-stroke','cancel-outer')})){
 foreach($mode in @('cancellation-disabled','event-receipt-false')){
  $after=$case.after|ConvertTo-Json -Depth 8|ConvertFrom-Json
  if($mode -eq 'cancellation-disabled'){$after.prevented=$false}else{$after.raw=$after.raw.Replace(' prevented=true]',' prevented=false]')}
  Assert ($case.name+': '+$mode+' rejected') (!(Test-Cancellation $after $true))
 }
}
$path=[IO.Path]::GetTempFileName()
try{
 Write-ProbeReport $path $retained;$roundtrip=Read-ProbeReport $path
 Assert 'Depth12 preserves all retained nested wheel data' (($retained|ConvertTo-Json -Depth 30 -Compress) -ceq ($roundtrip|ConvertTo-Json -Depth 30 -Compress))
}finally{Remove-Item $path -Force}
}
@{passed=$true;checks=$checks.Count;results=$checks;runnerSha256=(Get-FileHash $Runner).Hash;retainedResultSha256=if($RetainedResult){(Get-FileHash $RetainedResult).Hash}else{$null};scope='Pure helpers with synthetic windows/processes; no application or input'}|ConvertTo-Json -Depth 8
