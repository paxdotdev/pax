param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $Runner).Path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
$functions=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq 'Read-ProbeLog'},$true))
if($functions.Count -ne 1){throw 'Expected one shared log reader'}
. ([scriptblock]::Create($functions[0].Extent.Text))
$directory=Join-Path ([IO.Path]::GetTempPath()) ('pax-log-sharing-'+[Guid]::NewGuid().ToString('N'));[void][IO.Directory]::CreateDirectory($directory)
$path=Join-Path $directory 'native-host.log';$seed="Pax $([char]0x2014) $([char]0x03b1)`nfirst_frame_published`n";$handle=$null;$task=$null;$checks=@()
function Assert([string]$name,[bool]$ok){$script:checks+=@{name=$name;passed=$ok};if(!$ok){throw $name}}
function Cause($error){$cause=$error.Exception;while($cause.InnerException){$cause=$cause.InnerException};return $cause}
try{
 [IO.File]::WriteAllText($path,$seed,[Text.UTF8Encoding]::new($true))
 Assert 'Complete Unicode log and publication marker' ((Read-ProbeLog $path) -ceq $seed)
 $handle=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::Write,[IO.FileShare]::ReadWrite)
 $rejected=$false;try{[void][IO.File]::ReadAllText($path)}catch{$cause=Cause $_;$rejected=$cause -is [IO.IOException] -and (($cause.HResult -band 0xffff) -in @(32,33))}
 Assert 'Legacy reader reproduces append-handle sharing failure' $rejected
 Assert 'Shared reader coexists with held append handle' ((Read-ProbeLog $path) -ceq $seed)
 $handle.Dispose();$handle=$null
 $handle=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
 Assert 'Reader closes its handle before returning' ($handle -ne $null)
 $rejected=$false;$watch=[Diagnostics.Stopwatch]::StartNew()
 try{[void](Read-ProbeLog $path)}catch{$cause=Cause $_;$rejected=$cause -is [IO.IOException] -and (($cause.HResult -band 0xffff) -in @(32,33))}
 $watch.Stop();Assert 'Persistent incompatible handle fails after bounded retries' ($rejected -and $watch.Elapsed.TotalMilliseconds -ge 160 -and $watch.Elapsed.TotalSeconds -lt 3)
 $handle.Dispose();$handle=$null;Assert 'Failed read leaves original log unchanged' ((Read-ProbeLog $path) -ceq $seed)
 Add-Type @'
using System;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
public static class PaxLogSharingTest {
 public static void CloseAfter(IDisposable handle,int ms) { ThreadPool.QueueUserWorkItem(delegate(object x) { Thread.Sleep(ms);handle.Dispose(); }); }
 public static Task Append(string path) { return Task.Run(delegate { using(var f=new FileStream(path,FileMode.Open,FileAccess.Write,FileShare.ReadWrite|FileShare.Delete)) { f.Seek(0,SeekOrigin.End);for(int i=0;i<80;i++){var b=Encoding.UTF8.GetBytes("append="+i+"\n");f.Write(b,0,b.Length);f.Flush();Thread.Sleep(2);} } }); }
}
'@
 $handle=[IO.FileStream]::new($path,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None);[PaxLogSharingTest]::CloseAfter($handle,80)
 $watch=[Diagnostics.Stopwatch]::StartNew();$text=Read-ProbeLog $path;$watch.Stop();$handle=$null
 Assert 'Transient incompatible handle permits successful bounded retry' ($text -ceq $seed -and $watch.Elapsed.TotalMilliseconds -ge 40 -and $watch.Elapsed.TotalSeconds -lt 3)
 $rejected=$false;$watch=[Diagnostics.Stopwatch]::StartNew();try{[void](Read-ProbeLog (Join-Path $directory 'missing.log'))}catch{$rejected=(Cause $_) -is [IO.FileNotFoundException]};$watch.Stop()
 Assert 'Missing file error propagates without sharing retries' ($rejected -and $watch.Elapsed.TotalMilliseconds -lt 160)
 $rejected=$false;$watch=[Diagnostics.Stopwatch]::StartNew();try{[void](Read-ProbeLog $directory)}catch{$rejected=(Cause $_) -is [UnauthorizedAccessException]};$watch.Stop()
 Assert 'Access error propagates without sharing retries' ($rejected -and $watch.Elapsed.TotalMilliseconds -lt 160)
 $task=[PaxLogSharingTest]::Append($path);$observed=0
 for($i=0;$i -lt 100;$i++){$text=Read-ProbeLog $path;if(!$text.StartsWith($seed)){throw 'Concurrent append changed the stable Unicode prefix or publication marker'};$observed++;Start-Sleep -Milliseconds 2}
 Assert 'Concurrent append completes within bound' ($task.Wait(3000))
 Assert 'One hundred shared reads retain complete stable prefix' ($observed -eq 100)
 Assert 'Final appended log remains readable' ((Read-ProbeLog $path).EndsWith("append=79`n"))
 @{passed=$true;checks=$checks.Count;results=$checks;runnerSha256=(Get-FileHash $Runner).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 5
}finally{if($handle){$handle.Dispose()};if($task){[void]$task.Wait(3000)};if(Test-Path $directory){Remove-Item $directory -Recurse -Force}}
