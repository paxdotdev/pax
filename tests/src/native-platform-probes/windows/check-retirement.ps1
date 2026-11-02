param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'),[string]$RetainedLog='')
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $Runner).Path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
$functions=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq 'Get-ProbeRetirement'},$true))
if($functions.Count -ne 1){throw 'Expected one terminal retirement helper'};. ([scriptblock]::Create($functions[0].Extent.Text))
$good=@{allocatedBytes=0;allocatedImages=0;pendingBytes=0;pendingFrames=0;published=85;retired=85;closing=$true}
function Log($gpu){return '100 initialized_gpu={"allocatedBytes":0}'+"`n"+'200 closed_gpu='+($gpu|ConvertTo-Json -Compress)+"`n"+'200 closed exports_drained=true'+"`n"}
$cases=@(@{name='Exact terminal close passes';log=(Log $good);passed=$true})
foreach($key in @('allocatedBytes','allocatedImages','pendingBytes','pendingFrames')){$bad=$good.Clone();$bad[$key]=1;$cases+=@{name="Earlier initialization zero does not hide terminal $key";log=(Log $bad);passed=$false}}
$bad=$good.Clone();$bad.allocatedBytes=5;$cases+=@{name='Earlier successful close does not hide later nonzero close';log=((Log $good)+(Log $bad));passed=$false}
$bad=$good.Clone();$bad.retired=84;$cases+=@{name='Unretired publication fails';log=(Log $bad);passed=$false}
$bad=$good.Clone();$bad.closing=$false;$cases+=@{name='Non-closing terminal record fails';log=(Log $bad);passed=$false}
$bad=$good.Clone();$bad.Remove('pendingFrames');$cases+=@{name='Missing required counter fails';log=(Log $bad);passed=$false}
$bad=$good.Clone();$bad.allocatedBytes='0';$cases+=@{name='String counter is not accepted as zero';log=(Log $bad);passed=$false}
$bad=$good.Clone();$bad.published=-1;$bad.retired=-1;$cases+=@{name='Negative publication counts fail';log=(Log $bad);passed=$false}
$bad=$good.Clone();$bad.published=1.5;$bad.retired=1.5;$cases+=@{name='Fractional publication counts fail';log=(Log $bad);passed=$false}
$cases+=@{name='Initialization zero and close marker cannot replace terminal GPU record';log="100 initialized_gpu={`"allocatedBytes`":0}`n200 closed exports_drained=true`n";passed=$false}
$cases+=@{name='Malformed final GPU JSON fails';log=((Log $good)+"300 closed_gpu={bad}`n300 closed exports_drained=true`n");passed=$false}
$cases+=@{name='Missing final drained marker fails';log=('200 closed_gpu='+($good|ConvertTo-Json -Compress));passed=$false}
$cases+=@{name='Marker before final GPU record fails';log=("100 closed exports_drained=true`n200 closed_gpu="+($good|ConvertTo-Json -Compress));passed=$false}
$cases+=@{name='False drained marker fails';log=((Log $good).Replace('exports_drained=true','exports_drained=false'));passed=$false}
$cases+=@{name='Later error invalidates final close marker';log=((Log $good)+"300 error late failure`n");passed=$false}
$checks=@();foreach($case in $cases){$v=Get-ProbeRetirement $case.log;if($v.passed -ne $case.passed){throw ('Retirement control failed: '+$case.name)};$checks+=@{name=$case.name;passed=$true;expected=$case.passed;observed=$v.passed;error=$v.error}}
$result=@{passed=$true;checks=$checks.Count;results=$checks;runnerSha256=(Get-FileHash $Runner).Hash;utc=[DateTime]::UtcNow.ToString('o')}
if($RetainedLog){$result.retainedLog=@{path=$RetainedLog;sha256=(Get-FileHash $RetainedLog).Hash;retirement=(Get-ProbeRetirement ([IO.File]::ReadAllText($RetainedLog)));scope='Separate revalidation; historical report and helper bytes are unchanged'}}
$result|ConvertTo-Json -Depth 10
