param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $Runner).Path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
$definitions=@{}
foreach($name in @('Write-ProbeReport','Read-ProbeReport')){
 $functions=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name},$true))
 if($functions.Count -ne 1){throw ('Expected one helper: '+$name)}
 $definitions[$name]=$functions[0].Extent.Text;. ([scriptblock]::Create($definitions[$name]))
}
$directory=Join-Path ([IO.Path]::GetTempPath()) ('pax-report-sharing-'+[Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($directory);$path=Join-Path $directory 'result.json';$reader=$null;$job=$null;$checks=@()
function Assert([string]$name,[bool]$passed){$script:checks+=@{name=$name;passed=$passed};if(!$passed){throw $name}}
try{
 Write-ProbeReport $path @{sequence=0;unicode='Pax — α';payload=('x'*8192)}
 $initial=Read-ProbeReport $path;Assert 'Initial complete Unicode snapshot' ($initial.sequence -eq 0 -and $initial.unicode -ceq 'Pax — α')
 $reader=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
 Write-ProbeReport $path @{sequence=1;unicode='Pax — α';payload=('y'*8192)}
 Assert 'Replacement succeeds with compatible reader held' ((Read-ProbeReport $path).sequence -eq 1)
 $bytes=New-Object byte[] $reader.Length;[void]$reader.Read($bytes,0,$bytes.Length);$old=([Text.Encoding]::UTF8.GetString($bytes).TrimStart([char]0xfeff)|ConvertFrom-Json)
 Assert 'Held reader retains complete previous snapshot' ($old.sequence -eq 0 -and $old.payload -ceq ('x'*8192));$reader.Dispose();$reader=$null
 $reader=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
 $rejected=$false;$watch=[Diagnostics.Stopwatch]::StartNew()
 try{Write-ProbeReport $path @{sequence=2}}catch{$cause=$_.Exception;while($cause.InnerException){$cause=$cause.InnerException};$rejected=$cause -is [IO.IOException] -and (($cause.HResult -band 0xffff) -in @(32,33))}
 $watch.Stop();Assert 'Incompatible reader fails after bounded sharing retries' ($rejected -and $watch.Elapsed.TotalSeconds -lt 3)
 Assert 'Failed replacement preserves prior report' ((Read-ProbeReport $path).sequence -eq 1)
 Assert 'Failed replacement retires temporary file' (@(Get-ChildItem $directory -Filter '*.tmp').Count -eq 0)
 $reader.Dispose();$reader=$null;Write-ProbeReport $path @{sequence=2;payload=('z'*8192)};Assert 'Publication recovers after blocking reader closes' ((Read-ProbeReport $path).sequence -eq 2)
 Add-Type 'public static class PaxReportReaderRelease { public static void CloseAfter(System.IDisposable reader, int milliseconds) { System.Threading.ThreadPool.QueueUserWorkItem(delegate(object ignored) { System.Threading.Thread.Sleep(milliseconds); reader.Dispose(); }); } }'
 $reader=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
 [PaxReportReaderRelease]::CloseAfter($reader,80);$watch=[Diagnostics.Stopwatch]::StartNew()
 Write-ProbeReport $path @{sequence=2;payload=('z'*8192);retried=$true};$watch.Stop();$reader=$null
 Assert 'Transient legacy reader permits bounded successful retry' ((Read-ProbeReport $path).retried -eq $true -and $watch.Elapsed.TotalSeconds -lt 3)
 $ready=Join-Path $directory 'ready'
 $job=Start-Job -ArgumentList $path,$ready,$definitions['Read-ProbeReport'] -ScriptBlock {
  param($path,$ready,$definition);$ErrorActionPreference='Stop';. ([scriptblock]::Create($definition));[IO.File]::WriteAllText($ready,'ready');$observed=0
  for($i=0;$i -lt 300;$i++){$v=Read-ProbeReport $path;if($v.sequence -lt 2 -or $v.sequence -gt 101 -or $v.payload.Length -ne 8192){throw 'Partial or invalid concurrent report'};$observed++;Start-Sleep -Milliseconds 5}
  @{observed=$observed;passed=$true}
 }
 $until=[DateTime]::UtcNow.AddSeconds(10);while(!(Test-Path $ready) -and [DateTime]::UtcNow -lt $until){Start-Sleep -Milliseconds 20};Assert 'Concurrent reader starts within bound' (Test-Path $ready)
 for($i=3;$i -le 101;$i++){Write-ProbeReport $path @{sequence=$i;payload=('w'*8192)};Start-Sleep -Milliseconds 5}
 [void](Wait-Job $job -Timeout 10);Assert 'Concurrent reader finishes within bound' ($job.State -eq 'Completed')
 $observed=Receive-Job $job -ErrorAction Stop;Assert 'All concurrent snapshots are complete' ($observed.passed -and $observed.observed -eq 300)
 Assert 'Final publication remains complete' ((Read-ProbeReport $path).sequence -eq 101)
 @{passed=$true;checks=$checks.Count;results=$checks;runnerSha256=(Get-FileHash $Runner).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 5
}finally{if($reader){$reader.Dispose()};if($job){Stop-Job $job -ErrorAction SilentlyContinue;Remove-Job $job -Force -ErrorAction SilentlyContinue};if(Test-Path $directory){Remove-Item $directory -Recurse -Force}}
