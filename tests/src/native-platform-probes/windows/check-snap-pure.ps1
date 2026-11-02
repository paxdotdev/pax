param([Parameter(Mandatory=$true)][string]$Runner)
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile($Runner,[ref]$tokens,[ref]$errors);if($errors.Count){throw ($errors|Out-String)}
foreach($name in @('Measure-NativeOffsets','Near')){$found=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name},$true));if($found.Count -ne 1){throw 'Missing pure helper'};. ([scriptblock]::Create($found[0].Extent.Text))}
$x=@{id='pax-node-24';class='ScrollViewer';scrollX=$true;scrollY=$false;percentX=50.0;percentY=-1.0;viewX=(100.0/3.0);viewY=100.0;width=450.0;height=240.0}
$y=@{id='pax-node-40';class='ScrollViewer';scrollX=$false;scrollY=$true;percentX=-1.0;percentY=25.0;viewX=100.0;viewY=(100.0/3.0);width=340.0;height=240.0}
$checks=@();$value=Measure-NativeOffsets @($x,$y)
if([Math]::Abs($value.x-450) -gt .00001 -or [Math]::Abs($value.y-120) -gt .00001){throw 'Positive native geometry calculation'};$checks+=@{name='Distinct axes preserve raw-percent-to-offset formula';passed=$true}
foreach($case in @('duplicate-id','duplicate-axis','wrong-class','empty-id','missing-axis','nonfinite','zero-view')){
 $a=$x.Clone();$b=$y.Clone();$peers=@($a,$b)
 switch($case){'duplicate-id'{$b.id=$a.id};'duplicate-axis'{$b.scrollX=$true;$b.scrollY=$false};'wrong-class'{$b.class='Pane'};'empty-id'{$b.id=''};'missing-axis'{$peers=@($a)};'nonfinite'{$a.percentX=[double]::NaN};'zero-view'{$b.viewY=0}}
 $rejected=$false;try{$null=Measure-NativeOffsets $peers}catch{$rejected=$true};if(!$rejected){throw ('Negative geometry control accepted '+$case)};$checks+=@{name=$case;passed=$true}
}
if(!(Near 125 @(0,125,325,625)) -or (Near 173 @(0,125,325,625)) -or (Near 1.1 @(0))){throw 'Anchor positive/negative controls'}
$checks+=@{name='Exact anchor accepted and off-anchor/tolerance boundary rejected';passed=$true}
@{passed=$true;checks=$checks.Count;results=$checks;runnerSha256=(Get-FileHash $Runner).Hash;scope='Pure geometry/identity predicates only; no app, input, offset setter, or GPU';utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 5
