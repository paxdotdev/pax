param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'),[string]$Companion=(Join-Path $PSScriptRoot 'website-acceptance.ps1'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
# Extract only pure geometry/ownership helpers; never launch or inject input.
foreach($pair in @(@($Runner,@('Get-IntegerClickPoint')),@($Companion,@('Get-UnobscuredSourcePoint','Select-SourceChromeExclusions')))){
 $tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $pair[0]).Path,[ref]$tokens,[ref]$errors)
 if($errors.Count){throw ($errors|Out-String)}
 foreach($name in $pair[1]){
  $found=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name},$true))
  if($found.Count -ne 1){throw ('Expected one pure helper '+$name)};. ([scriptblock]::Create($found[0].Extent.Text))
 }
}
function Rect($e){[pscustomobject]@{Left=$e[0];Top=$e[1];Right=$e[2];Bottom=$e[3]}}
function Inside($point,$rect){$point.x -ge $rect.Left -and $point.x -lt $rect.Right -and $point.y -ge $rect.Top -and $point.y -lt $rect.Bottom}
$cases=@(
 @{name='observed right-edge vertical chrome';label=@(1282,155,1298,171);viewport=@(1018,139,1298,187);bars=,@(1285,42,1297,1000);expected=@(1283,163)},
 @{name='one-pixel left remainder';label=@(1284,155,1298,171);viewport=@(1018,139,1298,187);bars=,@(1285,42,1297,1000);expected=@(1284,163)},
 @{name='one-pixel right remainder';label=@(1286,155,1298,171);viewport=@(1018,139,1298,187);bars=,@(1285,42,1297,1000);expected=@(1297,163)},
 @{name='fully obscured label continues reveal';label=@(1285,155,1297,171);viewport=@(1018,139,1298,187);bars=,@(1285,42,1297,1000);expected=$null},
 @{name='bottom scrollbar';label=@(0,0,20,20);viewport=@(0,0,20,20);bars=,@(0,18,20,20);expected=@(10,9)},
 @{name='two perpendicular scrollbars';label=@(0,0,20,20);viewport=@(0,0,20,20);bars=@(@(18,0,20,20),@(0,18,20,20));expected=@(9,9)},
 @{name='nonoverlapping chrome';label=@(0,0,20,20);viewport=@(0,0,20,20);bars=,@(20,0,30,20);expected=@(10,10)},
 @{name='negative coordinates';label=@(-6,-4,4,6);viewport=@(-6,-4,4,6);bars=,@(-2,-4,2,6);expected=@(-4,1)},
 @{name='fractional obscured slice without integer';label=@(.2,0,1.2,10);viewport=@(0,0,20,20);bars=,@(1,0,20,20);expected=$null},
 @{name='fractional one-integer remainder';label=@(.2,0,2.2,10);viewport=@(0,0,20,20);bars=,@(1.2,0,20,20);expected=@(1,5)},
 @{name='label clipped to viewport before subtraction';label=@(-10,0,30,20);viewport=@(0,0,20,20);bars=,@(18,0,20,20);expected=@(9,10)},
 @{name='overlapping exclusions preserve complete coverage';label=@(0,0,20,20);viewport=@(0,0,20,20);bars=@(@(10,0,20,20),@(8,0,18,20));expected=@(4,10)}
)
$results=@()
foreach($case in $cases){
 $label=Rect $case.label;$viewport=Rect $case.viewport;$bars=@($case.bars|ForEach-Object{Rect $_})
 $hit=Get-UnobscuredSourcePoint $label $viewport $bars
 if($null -eq $case.expected){if($null -ne $hit){throw ('Expected no clickable integer: '+$case.name)}}else{
  if($null -eq $hit -or $hit.point.x -ne $case.expected[0] -or $hit.point.y -ne $case.expected[1]){throw ('Unexpected point: '+$case.name)}
  if(!(Inside $hit.point $label) -or !(Inside $hit.point $viewport)){throw ('Point is outside label/viewport: '+$case.name)}
  foreach($bar in $bars){if(Inside $hit.point $bar){throw ('Point still intersects native chrome: '+$case.name)}}
 }
 # Independent finite grid oracle: a null result must not discard an available
 # integer point, and a selected point must not be manufactured from an empty set.
 $count=0
 for($x=[int][Math]::Ceiling([Math]::Max($label.Left,$viewport.Left));$x -lt [Math]::Min($label.Right,$viewport.Right);$x++){
  for($y=[int][Math]::Ceiling([Math]::Max($label.Top,$viewport.Top));$y -lt [Math]::Min($label.Bottom,$viewport.Bottom);$y++){
   $point=@{x=$x;y=$y};$blocked=$false;foreach($bar in $bars){if(Inside $point $bar){$blocked=$true}}
   if(!$blocked){$count++}
  }
 }
 if(($count -eq 0) -ne ($null -eq $hit)){throw ('Independent grid coverage mismatch: '+$case.name)}
 $results+=@{name=$case.name;passed=$true;hit=$hit;availableIntegerPoints=$count}
}
function Reject([string]$name,[scriptblock]$body){$rejected=$false;try{&$body|Out-Null}catch{$rejected=$true};if(!$rejected){throw ('Did not reject '+$name)};$script:results+=@{name=$name;passed=$true}}
$label=Rect @(0,0,20,20)
Reject 'nonfinite scrollbar geometry' {Get-UnobscuredSourcePoint $label $label @((Rect @([double]::NaN,0,10,20)))}
Reject 'empty scrollbar geometry' {Get-UnobscuredSourcePoint $label $label @((Rect @(0,0,0,20)))}
$owner=@{id='pax-node-34';bounds=(Rect @(18,41,1298,1001));horizontal=$false;vertical=$true}
$bar=@{id='VerticalScrollBar';ownerId='pax-node-34';orientation='Vertical';offscreen=$false;enabled=$true;bounds=(Rect @(1285,42,1297,1000))}
$valid=@(Select-SourceChromeExclusions $owner @($bar));if($valid.Count -ne 1){throw 'Owned visible scrollbar was not excluded'}
$results+=@{name='owned visible native scrollbar';passed=$true}
$hidden=$bar.Clone();$hidden.offscreen=$true;$hidden.bounds=$null
if(@(Select-SourceChromeExclusions $owner @($hidden)).Count -ne 0){throw 'Hidden empty scrollbar invented an exclusion'}
$results+=@{name='hidden peer supplies inventory without invented geometry';passed=$true}
Reject 'missing scrollable-axis peer' {Select-SourceChromeExclusions $owner @()}
Reject 'duplicate scrollbar orientation' {Select-SourceChromeExclusions $owner @($bar,$bar)}
Reject 'non-native owner identity' {$bad=$owner.Clone();$bad.id='PART_ScrollPresenter';Select-SourceChromeExclusions $bad @($bar)}
Reject 'foreign ancestor scrollbar' {$bad=$bar.Clone();$bad.ownerId='pax-node-99';Select-SourceChromeExclusions $owner @($bad)}
Reject 'wrong scrollbar orientation' {$bad=$bar.Clone();$bad.orientation='Horizontal';Select-SourceChromeExclusions $owner @($bad)}
Reject 'unknown scrollbar identity' {$bad=$bar.Clone();$bad.id='Unknown';Select-SourceChromeExclusions $owner @($bad)}
Reject 'visible peer without geometry' {$bad=$bar.Clone();$bad.bounds=$null;Select-SourceChromeExclusions $owner @($bad)}
Reject 'scrollbar outside native owner' {$bad=$bar.Clone();$bad.bounds=Rect @(1285,42,1300,1000);Select-SourceChromeExclusions $owner @($bad)}
Reject 'nonfinite native owner bounds' {$bad=$owner.Clone();$bad.bounds=Rect @(18,41,[double]::PositiveInfinity,1001);Select-SourceChromeExclusions $bad @($bar)}
Reject 'unbounded direct peer inventory' {Select-SourceChromeExclusions $owner @($bar,$bar,$bar)}
# The old label-only center reproduces the intercepted point from the trace.
$legacy=Get-IntegerClickPoint (Rect @(1282,155,1298,171))
if($legacy.x -ne 1290 -or !(Inside $legacy $bar.bounds)){throw 'Legacy chrome-overlap negative control did not reproduce'}
$results+=@{name='legacy visible-label center intersects actual scrollbar';passed=$true}
@{passed=$true;checks=$results.Count;results=$results;runnerSha256=(Get-FileHash $Runner).Hash;companionSha256=(Get-FileHash $Companion).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 7
