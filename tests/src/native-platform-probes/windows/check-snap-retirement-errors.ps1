param([Parameter(Mandatory=$true)][string]$Runner)
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$t=$null;$e=$null;$ast=[Management.Automation.Language.Parser]::ParseFile($Runner,[ref]$t,[ref]$e);if($e.Count){throw ($e|Out-String)}
foreach($name in @('Get-ProbeRetirement','Read-RetirementEvidence')){$f=@($ast.FindAll({param($n)$n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -ceq $name},$true));if($f.Count -ne 1){throw 'Expected retirement helper'};. ([scriptblock]::Create($f[0].Extent.Text))}
$path=[IO.Path]::GetTempFileName();$checks=@()
function Read-ProbeLog([string]$unused){if($script:failRead){throw 'synthetic final log sharing failure'};return $script:fixtureText}
try{
 foreach($case in @('valid-terminal','passing-run-read-error','original-failure-read-error','missing-terminal')){
  $report=[ordered]@{result='passed';cleanRetirement=$false};$script:failRead=$case -like '*read-error';$script:fixtureText="100 closed_gpu={`"allocatedBytes`":0,`"allocatedImages`":0,`"pendingBytes`":0,`"pendingFrames`":0,`"published`":85,`"retired`":85,`"closing`":true}`n101 closed exports_drained=true`n"
  if($case -eq 'original-failure-read-error'){$report.result='failed';$report.error='original input failure'};if($case -eq 'missing-terminal'){$script:fixtureText='100 initialized_gpu={"allocatedBytes":0}'}
  Read-RetirementEvidence $path
  $ok=switch($case){'valid-terminal'{$report.result -eq 'passed' -and $report.cleanRetirement};'passing-run-read-error'{$report.result -eq 'failed' -and !$report.cleanRetirement -and $report.error -match 'synthetic final log'};'original-failure-read-error'{$report.result -eq 'failed' -and !$report.cleanRetirement -and $report.error -ceq 'original input failure' -and $report.retirementLogReadError -match 'synthetic final log'};'missing-terminal'{$report.result -eq 'failed' -and !$report.cleanRetirement}}
  if(!$ok){throw ('Retirement error control failed '+$case)};$checks+=@{name=$case;passed=$true}
 }
 @{passed=$true;checks=$checks.Count;results=$checks;runnerSha256=(Get-FileHash $Runner).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 5
}finally{Remove-Item $path -Force}
