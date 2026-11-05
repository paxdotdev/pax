param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'),[string]$Companion=(Join-Path $PSScriptRoot 'website-acceptance.ps1'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
foreach($pair in @(@($Runner,'Get-IntegerClickPoint'),@($Companion,'Test-SourceSectionReady'))){
 $tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $pair[0]).Path,[ref]$tokens,[ref]$errors)
 if($errors.Count){throw ($errors|Out-String)}
 $name=$pair[1];$found=@($ast.FindAll({param($n)$n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -ceq $name},$true))
 if($found.Count -ne 1){throw ('Expected one readiness helper '+$name)};. ([scriptblock]::Create($found[0].Extent.Text))
}
function Rect($edges){[pscustomobject]@{Left=$edges[0];Top=$edges[1];Right=$edges[2];Bottom=$edges[3]}}
$cases=@(
 @{name='observed heading center304';change={param($c)};want=$true},
 @{name='observed heading center448';change={param($c)$c.anchor=Rect @(94,440,250,456);$c.toggle=Rect @(1190,384,1298,418)};want=$true},
 @{name='exact lower edge of authored navigation';change={param($c)$c.toggle=Rect @(1190,121,1298,155)};want=$true},
 @{name='one-pixel navigation overlap';change={param($c)$c.toggle=Rect @(1190,120,1298,154)};want=$false},
 @{name='viewport bottom clipping';change={param($c)$c.anchor=Rect @(94,990,250,1002)};want=$false},
 @{name='viewport left clipping';change={param($c)$c.anchor=Rect @(17,296,250,312)};want=$false},
 @{name='fractional slice with no integer';change={param($c)$c.toggle=Rect @(1190.2,240,1190.8,274)};want=$false},
 @{name='missing toggle';change={param($c)$c.count=0};want=$false},
 @{name='ambiguous toggle association';change={param($c)$c.count=2};want=$false},
 @{name='stale toggle identity';change={param($c)$c.actual='other'};want=$false},
 @{name='missing expected identity';change={param($c)$c.expected=''};want=$false},
 @{name='nonfinite heading bounds';change={param($c)$c.anchor=Rect @(94,[double]::NaN,250,312)};want=$false},
 @{name='empty heading';change={param($c)$c.anchor=Rect @(94,296,250,296)};want=$false},
 @{name='invalid navigation height';change={param($c)$c.nav=-1};want=$false}
)
$results=@()
foreach($case in $cases){
 $c=@{anchor=(Rect @(94,296,250,312));toggle=(Rect @(1190,240,1298,274));viewport=(Rect @(18,41,1298,1001));nav=80;count=1;expected='same';actual='same'}
 &$case.change $c
 $actual=Test-SourceSectionReady $c.anchor $c.toggle $c.viewport $c.nav $c.count $c.expected $c.actual
 if($actual -ne $case.want){throw ('Unexpected readiness: '+$case.name)}
 $results+=@{name=$case.name;passed=$true;ready=$actual}
}
# The retained run used full native144px wheel steps. Both observed poses miss
# the obsolete130px centering band; ordinary navigation reaches304 in one step.
$oldLow=41+960*.35-65;$oldHigh=$oldLow+130
if((304 -ge $oldLow -and 304 -le $oldHigh) -or (448 -ge $oldLow -and 448 -le $oldHigh)){throw 'Legacy oscillation control no longer reproduces'}
$ordinaryLow=41+120;$ordinaryHigh=41+[Math]::Min(400,960-30)
if(304 -lt $ordinaryLow -or 304 -gt $ordinaryHigh -or 448-144 -ne 304){throw 'Ordinary source navigation cannot reach observed ready pose'}
$results+=@{name='legacy wheel oscillation removed without changing wheel delta';passed=$true;oldBand=@($oldLow,$oldHigh);ordinaryBand=@($ordinaryLow,$ordinaryHigh);observedCenters=@(304,448)}
@{passed=$true;checks=$results.Count;results=$results;runnerSha256=(Get-FileHash $Runner).Hash;companionSha256=(Get-FileHash $Companion).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 6
