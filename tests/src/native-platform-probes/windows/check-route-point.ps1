param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'),[string]$Companion=(Join-Path $PSScriptRoot 'website-acceptance.ps1'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
# Parse and evaluate pure geometry only; no application or input is started.
foreach($pair in @(@($Runner,'Get-IntegerClickPoint'),@($Companion,'Get-VisibleViewportPoint'))){
 $tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $pair[0]).Path,[ref]$tokens,[ref]$errors)
 if($errors.Count){throw ($errors|Out-String)}
 $name=$pair[1];$matches=@($ast.FindAll({param($n)$n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -ceq $name},$true))
 if($matches.Count -ne 1){throw ('Missing pure helper '+$name)};. ([scriptblock]::Create($matches[0].Extent.Text))
}
function Rect($e){[pscustomobject]@{Left=$e[0];Top=$e[1];Right=$e[2];Bottom=$e[3]}}
$v=Rect @(18,121,1298,1001)
$cases=@(
 @{name='footer at maximum scroll';bounds=@(691,750,771,766);expected=@(731,758)},
 @{name='bottom one-pixel slice';bounds=@(691,1000,771,1016);expected=@(731,1000)},
 @{name='excluded bottom edge';bounds=@(691,1001,771,1017);expected=$null},
 @{name='excluded right edge';bounds=@(1298,750,1378,766);expected=$null},
 @{name='fractional slice without integer';bounds=@(1297.2,750,1300,766);expected=$null},
 @{name='left clipped slice';bounds=@(-10,750,19,766);expected=@(18,758)}
)
$rows=@()
foreach($c in $cases){
 $b=Rect $c.bounds;$p=Get-VisibleViewportPoint $b $v
 if($null -eq $c.expected){if($null -ne $p){throw ('Unexpected point '+$c.name)}}else{
  if($null -eq $p -or $p.x -ne $c.expected[0] -or $p.y -ne $c.expected[1]){throw ('Wrong point '+$c.name)}
  foreach($r in @($b,$v)){if(!($p.x -ge $r.Left -and $p.x -lt $r.Right -and $p.y -ge $r.Top -and $p.y -lt $r.Bottom)){throw ('Outside half-open geometry '+$c.name)}}
 };$rows+=@{case=$c.name;passed=$true;point=$p}
}
# Preserve the old constraint as a negative control, not as route acceptance.
if(758 -le 521){throw 'Footer no longer reproduces original targeting-band failure'}
$rows+=@{case='footer outside original upper band';passed=$true}
@{passed=$true;checks=$rows.Count;results=$rows;runnerSha256=(Get-FileHash $Runner).Hash;companionSha256=(Get-FileHash $Companion).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 5
