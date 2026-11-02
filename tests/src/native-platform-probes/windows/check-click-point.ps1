param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
# Evaluate only the pure coordinate helper; never launch an application.
$tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $Runner).Path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
$functions=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq 'Get-IntegerClickPoint'},$true))
if($functions.Count -ne 1){throw 'Expected one coordinate helper'}
. ([scriptblock]::Create($functions[0].Extent.Text))
$cases=@(
 @{name='odd one-pixel edge';edges=@(1273,403,1274,404);expected=@(1273,403)},
 @{name='even one-pixel edge';edges=@(1274,404,1275,405);expected=@(1274,404)},
 @{name='original 24-pixel frontier';edges=@(1250,477,1274,492);expected=@(1262,484)},
 @{name='odd width';edges=@(1251,477,1274,492);expected=@(1262,484)},
 @{name='fractional origin';edges=@(.2,-.8,1.2,.2);expected=@(1,0)},
 @{name='negative origin';edges=@(-4,-5,-1,-2);expected=@(-3,-4)},
 @{name='negative fractional origin';edges=@(-1.7,-3.4,-.2,-1.1);expected=@(-1,-3)},
 @{name='positive subpixel without integer';edges=@(.2,0,.8,10);expected=$null},
 @{name='negative subpixel without integer';edges=@(-.8,0,-.2,10);expected=$null},
 @{name='subpixel height without integer';edges=@(0,.2,10,.8);expected=$null},
 @{name='zero width';edges=@(1,0,1,10);expected=$null},
 @{name='reversed edges';edges=@(2,0,1,10);expected=$null},
 @{name='nonfinite edge';edges=@([double]::NaN,0,1,10);expected=$null},
 @{name='infinite edge';edges=@(0,0,[double]::PositiveInfinity,10);expected=$null}
)
$results=@()
foreach($case in $cases){
 $e=$case.edges;$bounds=[pscustomobject]@{Left=$e[0];Top=$e[1];Right=$e[2];Bottom=$e[3]};$point=Get-IntegerClickPoint $bounds
 if($null -eq $case.expected){if($null -ne $point){throw ('Expected no integer point: '+$case.name)}}else{
  if($null -eq $point -or $point.x -ne $case.expected[0] -or $point.y -ne $case.expected[1]){throw ('Unexpected point: '+$case.name)}
  if(!($point.x -ge $bounds.Left -and $point.x -lt $bounds.Right -and $point.y -ge $bounds.Top -and $point.y -lt $bounds.Bottom)){throw ('Point outside half-open bounds: '+$case.name)}
 }
 $results+=@{case=$case.name;passed=$true;point=$point}
}
# Negative controls reproduce the original rounding bug at each excluded edge.
$x=[int](1273+1/2);$y=[int](403+1/2)
if($x -lt 1274 -or $y -lt 404){throw 'Legacy negative control no longer reproduces edge rounding'}
$results+=@{case='legacy right-edge rounding rejected';passed=($x -ge 1274)}
$results+=@{case='legacy bottom-edge rounding rejected';passed=($y -ge 404)}
@{passed=$true;checks=$results.Count;results=$results;runnerSha256=(Get-FileHash $Runner).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 5
