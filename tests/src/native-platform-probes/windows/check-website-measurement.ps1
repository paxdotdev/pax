param([string]$Runner=(Join-Path $PSScriptRoot 'check-native-scroll.ps1'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest
$tokens=$null;$errors=$null;$ast=[Management.Automation.Language.Parser]::ParseFile((Resolve-Path $Runner).Path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
foreach($name in @('Get-WebsiteMeasurementPlan','Get-WebsiteDiagnosticEnvironment','Get-IntegerClickPoint','Get-WebsiteMeasurementPoint')){
 $functions=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name},$true))
 if($functions.Count -ne 1){throw "Expected one $name helper"};. ([scriptblock]::Create($functions[0].Extent.Text))
}
$checks=@();function Assert([string]$name,[bool]$ok){$script:checks+=@{name=$name;passed=$ok};if(!$ok){throw $name}}
$pointerCases=@(
 @{name='ordinary outer viewport';b=@{Left=18;Top=121;Right=1298;Bottom=1001};x=43;y=561},
 @{name='fractional viewport';b=@{Left=18.2;Top=121.5;Right=1298.2;Bottom=1001.5};x=43;y=561},
 @{name='negative origin';b=@{Left=-90;Top=-40;Right=10;Bottom=40};x=-88;y=0},
 @{name='one-pixel clipped viewport';b=@{Left=1273;Top=403;Right=1274;Bottom=404};x=1273;y=403}
)
foreach($case in $pointerCases){
 $point=Get-WebsiteMeasurementPoint $case.b
 Assert ("Measurement pointer: "+$case.name) ($null -ne $point -and $point.x -eq $case.x -and $point.y -eq $case.y -and $point.x -ge $case.b.Left -and $point.x -lt $case.b.Right -and $point.y -ge $case.b.Top -and $point.y -lt $case.b.Bottom)
}
Assert 'Measurement pointer rejects subpixel slice without integer' ($null -eq (Get-WebsiteMeasurementPoint @{Left=.1;Top=0;Right=.9;Bottom=1}))
Assert 'Measurement pointer rejects nonfinite viewport' ($null -eq (Get-WebsiteMeasurementPoint @{Left=0;Top=0;Right=[double]::PositiveInfinity;Bottom=1}))
$plan=@(Get-WebsiteMeasurementPlan 'memory')
Assert 'Memory plan has both stationary intervals and three complete round trips' ($plan.Count -eq 8 -and (($plan|ForEach-Object{$_.name}) -join ',') -ceq 'top-stationary,traversal-1-down,traversal-1-up,traversal-2-down,traversal-2-up,traversal-3-down,traversal-3-up,top-after-traversals')
Assert 'Memory direction order returns to top after every down leg' ((($plan|ForEach-Object{$_.direction}) -join ',') -ceq '0,-1,1,-1,1,-1,1,0')
Assert 'Memory intervals are ninety seconds with bounded traversals' (@($plan|Where-Object{$_.seconds -ne 90}).Count -eq 0)
$plan=@(Get-WebsiteMeasurementPlan 'stationary')
Assert 'ABBA launch has exactly one forty-five-second stationary interval' ($plan.Count -eq 1 -and $plan[0].name -ceq 'top-stationary' -and $plan[0].direction -eq 0 -and $plan[0].seconds -eq 45)
Assert 'Default mode adds no measurement phases' (@(Get-WebsiteMeasurementPlan 'none').Count -eq 0)
$rejected=$false;try{[void](Get-WebsiteMeasurementPlan 'bad')}catch{$rejected=$true};Assert 'Unknown plan is rejected' $rejected
$flags=Get-WebsiteDiagnosticEnvironment 'existing';Assert 'Default flag behavior is unchanged' ($flags.Count -eq 1 -and $flags.PAX_WINDOWS_TIMINGS -ceq '1')
foreach($mode in @('enabled','disabled')){
 $flags=Get-WebsiteDiagnosticEnvironment $mode;$expected=if($mode -eq 'enabled'){'1'}else{'0'}
 Assert "$mode sets all four diagnostic flags explicitly" ($flags.Count -eq 5 -and $flags.PAX_WINDOWS_TIMINGS -ceq $expected -and $flags.PAX_GPU_MEMORY_DIAGNOSTICS -ceq $expected -and $flags.PAX_RENDER_TIMINGS -ceq $expected -and $flags.PAX_PROPERTY_MEMORY_DIAGNOSTICS -ceq $expected)
 Assert "$mode keeps separate phase timing disabled" ($flags.PAX_WINDOWS_PHASE_TIMINGS -ceq '0')
}
$rejected=$false;try{[void](Get-WebsiteDiagnosticEnvironment 'bad')}catch{$rejected=$true};Assert 'Unknown diagnostic mode is rejected' $rejected
$text=[IO.File]::ReadAllText((Resolve-Path $Runner).Path);$boundary=$text.IndexOf('Add-Type -AssemblyName UIAutomationClient')
if($boundary -le 0){throw 'Pure validation boundary missing'};$probe=[scriptblock]::Create($text.Substring(0,$boundary)+'; "valid"')
$cases=@(
 @{name='ordinary website phases';flags=@{};valid=$true},
 @{name='memory enabled';flags=@{WebsiteMeasurement='memory';WebsiteDiagnosticsMode='enabled'};valid=$true},
 @{name='stationary disabled';flags=@{WebsiteMeasurement='stationary';WebsiteDiagnosticsMode='disabled';LaunchOrdinal=4};valid=$true},
 @{name='measurement requires website phases';flags=@{WebsiteMeasurement='memory';WebsitePhases=$false};valid=$false},
 @{name='measurement excludes source interaction';flags=@{WebsiteMeasurement='memory';OpenSource=$true};valid=$false},
 @{name='measurement excludes startup screenshot sequence';flags=@{WebsiteMeasurement='stationary';StartupSequence=$true};valid=$false},
 @{name='diagnostic override requires explicit measurement';flags=@{WebsiteDiagnosticsMode='disabled'};valid=$false},
 @{name='zero launch ordinal rejected';flags=@{WebsiteMeasurement='stationary';LaunchOrdinal=0};valid=$false},
 @{name='unknown declared measurement rejected';flags=@{WebsiteMeasurement='unknown'};valid=$false}
)
foreach($case in $cases){
 $arguments=@{Executable='unused.exe';OutputDirectory='unused';WebsitePhases=$true};foreach($entry in $case.flags.GetEnumerator()){$arguments[$entry.Key]=$entry.Value}
 $failed=$false;$value=$null;try{$value=& $probe @arguments}catch{$failed=$true}
 Assert $case.name $(if($case.valid){!$failed -and $value -ceq 'valid'}else{$failed})
}
# Exercise the real sampling function with deterministic process counters, without
# opening an application or a UIA connection. Brackets include trace observation.
$functions=@($ast.FindAll({param($node)$node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq 'Website-Sample'},$true))
if($functions.Count -ne 1){throw 'Expected one website sample function'};. ([scriptblock]::Create($functions[0].Extent.Text))
$process=[pscustomobject]@{HasExited=$false;MainWindowTitle='Pax';WorkingSet64=1024L;Id=123;TotalProcessorTime=[TimeSpan]::FromSeconds(2);PrivateMemorySize64=2048L;HandleCount=12;Threads=@(1,2)}
$process|Add-Member -MemberType ScriptMethod -Name Refresh -Value {Start-Sleep -Milliseconds 2}
$report=@{samples=@()};$outerPattern=[pscustomobject]@{Current=[pscustomobject]@{VerticalScrollPercent=0}};$siteWatch=[Diagnostics.Stopwatch]::StartNew()
function TraceOffsets {Start-Sleep -Milliseconds 2;return @{'native-host.log'=123}}
$a=Website-Sample 'stationary-start';$b=Website-Sample 'stationary-end'
Assert 'Samples have distinct consecutive IDs and exact process identity' ($a.sampleId -eq 0 -and $b.sampleId -eq 1 -and $a.pid -eq 123 -and $b.pid -eq 123)
Assert 'Sample UTC brackets include process refresh and trace reads' ($a.sampleFinishedUnixMs-$a.sampleStartedUnixMs -ge 2 -and $b.sampleStartedUnixMs -ge $a.sampleFinishedUnixMs -and $b.sampleFinishedUnixMs -ge $b.sampleStartedUnixMs)
Assert 'Samples retain exact counters and trace offsets' ($a.privateBytes -eq 2048 -and $a.workingSet -eq 1024 -and $a.cpuSeconds -eq 2 -and $a.traceOffsets['native-host.log'] -eq 123 -and $report.samples.Count -eq 2)
$process.HasExited=$true;$rejected=$false;try{[void](Website-Sample 'dead')}catch{$rejected=$true};Assert 'Exited process is not sampled as live' ($rejected -and $report.samples.Count -eq 2)
$process.HasExited=$false;$process.PrivateMemorySize64=8GB
$limit=Website-Sample 'at-private-bound'
Assert 'Exact private-byte bound remains sampleable' ($limit.privateBytes -eq 8GB -and $report.samples.Count -eq 3)
$process.PrivateMemorySize64=8GB+1;$rejected=$false
try{[void](Website-Sample 'over-private-bound')}catch{$rejected=$_.ToString() -like '*eight-GiB private-bytes*'}
Assert 'Private-byte excess with small working set preserves prior samples and fails' ($rejected -and $process.WorkingSet64 -eq 1024 -and $report.samples.Count -eq 3)
$process.PrivateMemorySize64=2048L;$process.WorkingSet64=6GB
$limit=Website-Sample 'at-working-set-bound'
Assert 'Exact working-set bound remains sampleable' ($limit.workingSet -eq 6GB -and $report.samples.Count -eq 4)
$process.WorkingSet64=6GB+1;$rejected=$false
try{[void](Website-Sample 'over-working-set-bound')}catch{$rejected=$_.ToString() -like '*six-GiB probe safety bound*'}
Assert 'Working-set excess still preserves prior samples and fails' ($rejected -and $report.samples.Count -eq 4)
@{passed=$true;checks=$checks.Count;results=$checks;runnerSha256=(Get-FileHash $Runner).Hash;utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 5
