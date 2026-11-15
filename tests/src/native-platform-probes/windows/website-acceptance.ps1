# Companion to check-native-scroll.ps1. Uses its owned input, lifecycle and logging
# helpers; never sets application properties or substitutes content.
function Invoke-WebsiteAcceptance {
 $report.websiteInteractionScope=if($WebsiteHeaderOnly){'header'}elseif($WebsiteRouteOnly){'route'}elseif($WebsiteSourcesOnly){'sources-and-route'}elseif($WebsiteDemosOnly){'demos'}else{'full'};$report.websiteInteractionIssues=@();$report.websiteInteractionSteps=@();$report.websiteSourceTabs=@();$report.websiteSourceClicks=@();$report.websiteClickObservations=@();$report.websiteRevealSamples=@();$report.websiteSourceChromeHits=@();Save
 $sourceRows=Get-Content $WebsiteSourceManifest -Raw|ConvertFrom-Json
 $sourceTruth=@{};foreach($row in $sourceRows){
  if($sourceTruth.ContainsKey($row.label) -and $sourceTruth[$row.label].displaySha256Utf8 -cne $row.displaySha256Utf8){throw ('Conflicting frozen source identity: '+$row.label)}
  $sourceTruth[$row.label]=$row
 }
 function Observe([string]$name,[bool]$ok,$detail){$report.checks+=@{name=$name;passed=$ok;detail=$detail};if(!$ok){$report.websiteInteractionIssues+=,$name};Save}
 function Visible-Item($item){$b=Visible-Scroll-Bounds $item;return (!$b.IsEmpty -and $b.Width -ge 3 -and $b.Height -ge 3)}
 function Named([string]$name,$controlType=[System.Windows.Automation.ControlType]::Text){
  $type=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$controlType)
  $label=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,$name)
  $match=New-Object System.Windows.Automation.AndCondition($type,$label)
  $items=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$match)
  for($i=0;$i -lt $items.Count;$i++){,$items.Item($i)}
 }
 function Visible-Named([string]$name,$controlType=[System.Windows.Automation.ControlType]::Text){return @(Named $name $controlType|Where-Object{Visible-Item $_})}
 function Text-Snapshot($container) {
  # Fetch only the current calculator's text, never the entire home page.
  if($null -eq $container -or $container.Current.ProcessId -ne $process.Id){throw 'Text observation has no owned native container'}
  $cache=New-Object System.Windows.Automation.CacheRequest
  $cache.Add([System.Windows.Automation.AutomationElement]::NameProperty)
  $cache.TreeScope=[System.Windows.Automation.TreeScope]::Element;$active=$cache.Activate()
  $match=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Text)
  try{foreach($item in $container.FindAll([System.Windows.Automation.TreeScope]::Descendants,$match)){[pscustomobject]@{item=$item;name=$item.Cached.Name}}}finally{$active.Dispose()}
 }
 function Tab-Snapshot($strip){
  # Clipped text peers may have Empty bounds. Fetch a fresh viewport-sized
  # observation in one cache request; never use offscreen bounds to order tabs.
  $cache=New-Object System.Windows.Automation.CacheRequest
  $cache.Add([System.Windows.Automation.AutomationElement]::NameProperty)
  $cache.Add([System.Windows.Automation.AutomationElement]::BoundingRectangleProperty)
  $cache.TreeScope=[System.Windows.Automation.TreeScope]::Element;$active=$cache.Activate()
  try{
   $match=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Text)
   $items=$strip.FindAll([System.Windows.Automation.TreeScope]::Descendants,$match)
   for($i=0;$i -lt $items.Count;$i++){$item=$items.Item($i);[pscustomobject]@{item=$item;name=$item.Cached.Name;bounds=$item.Cached.BoundingRectangle}}
  }finally{$active.Dispose()}
 }
 function Tab-Is-Visible($row,$viewport){
  $b=$row.bounds
  if($b.IsEmpty -or $viewport.IsEmpty){return $false}
  $width=[Math]::Min($b.Right,$viewport.Right)-[Math]::Max($b.Left,$viewport.Left)
  $height=[Math]::Min($b.Bottom,$viewport.Bottom)-[Math]::Max($b.Top,$viewport.Top)
  if($width -lt [Math]::Min(20,$b.Width) -or $height -lt 3){return $false}
  $b.Intersect($viewport)
  return $null -ne (Get-IntegerClickPoint $b)
 }
 function Click-Visible($item){
  # Read fresh bounds immediately before input. Authored animations need not
  # stop for a control to be clickable; Reveal keeps page targets below nav.
  $b=Visible-Scroll-Bounds $item;$point=Get-IntegerClickPoint $b
  if($null -eq $point){throw 'Requested item has no visible integer hit point'}
  $report.websiteClickObservations+=@{name=$item.Current.Name;bounds=$b.ToString();point=$point;utc=[DateTime]::UtcNow.ToString('o')};Save
  Point $point.x $point.y $true;Start-Sleep -Milliseconds 500
 }
 # Diagnostics are separate from acceptance: a failed frontier click remains a
 # failure even if a later, more interior physical click selects the right tab.
 function Source-Native-Scroller($item){
  # A template presenter can be the observed tab peer. Resolve its owning Pax
  # scroller instead of treating that internal peer as another authored node.
  for($depth=0;$null -ne $item -and $depth -lt 24;$depth++){
   if($item.Current.ProcessId -ne $process.Id){return $null}
   if($item.Current.AutomationId -like 'pax-node-*' -and $item.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsScrollPatternAvailableProperty)){return $item}
   $item=[System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($item)
  }
  return $null
 }
 function Source-Body-Patterns($codeScroll){
  # Only this drawer's native code scroller owns the document. Do not fetch
  # every offscreen Text.Name (including other large documents) in the page.
  if($null -eq $codeScroll -or $codeScroll.Current.ProcessId -ne $process.Id -or $codeScroll.Current.AutomationId -notlike 'pax-node-*'){throw 'Source code scroller identity is unavailable'}
  $match=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Text)
  foreach($item in $codeScroll.FindAll([System.Windows.Automation.TreeScope]::Descendants,$match)){
   $pattern=$null
   if($item.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern,[ref]$pattern)){,$pattern}
  }
 }
 function Get-UnobscuredSourcePoint($label,$viewport,$exclusions){
  $region=[pscustomobject]@{Left=[Math]::Max($label.Left,$viewport.Left);Top=[Math]::Max($label.Top,$viewport.Top);Right=[Math]::Min($label.Right,$viewport.Right);Bottom=[Math]::Min($label.Bottom,$viewport.Bottom)}
  if($null -eq (Get-IntegerClickPoint $region)){return $null}
  $regions=@($region)
  foreach($bar in $exclusions){
   foreach($edge in @($bar.Left,$bar.Top,$bar.Right,$bar.Bottom)){if([double]::IsNaN($edge) -or [double]::IsInfinity($edge)){throw 'Nonfinite native scrollbar bounds'}}
   if($bar.Right -le $bar.Left -or $bar.Bottom -le $bar.Top){throw 'Empty native scrollbar exclusion'}
   $next=@()
   foreach($r in $regions){
    $l=[Math]::Max($r.Left,$bar.Left);$t=[Math]::Max($r.Top,$bar.Top);$right=[Math]::Min($r.Right,$bar.Right);$bottom=[Math]::Min($r.Bottom,$bar.Bottom)
    if($right -le $l -or $bottom -le $t){$next+=,$r;continue}
    # Disjoint rectangles preserve every content pixel outside actual chrome.
    foreach($edges in @(@($r.Left,$r.Top,$l,$r.Bottom),@($right,$r.Top,$r.Right,$r.Bottom),@($l,$r.Top,$right,$t),@($l,$bottom,$right,$r.Bottom))){
     if($edges[2] -gt $edges[0] -and $edges[3] -gt $edges[1]){$next+=,[pscustomobject]@{Left=$edges[0];Top=$edges[1];Right=$edges[2];Bottom=$edges[3]}}
    }
   }
   if($next.Count -gt 128){throw 'Source chrome fragmentation bound exceeded'};$regions=$next
  }
  foreach($r in @($regions|Sort-Object @{Expression={($_.Right-$_.Left)*($_.Bottom-$_.Top)};Descending=$true},Left,Top)){
   $point=Get-IntegerClickPoint $r;if($null -ne $point){return @{point=$point;region=$r}}
  }
  return $null
 }
 function Select-SourceChromeExclusions($owner,$peers){
  if($owner.id -notmatch '^pax-node-[0-9]+$'){throw 'Scrollbar owner is not an authored native scroller'}
  if(@($peers).Count -gt 2){throw 'Ambiguous native scrollbar peer count'}
  $orientations=@{}
  foreach($bar in $peers){
   if($bar.ownerId -cne $owner.id){throw 'Scrollbar is not owned by the observed ancestor'}
   if($bar.orientation -notin @('Horizontal','Vertical') -or $bar.id -cne ($bar.orientation+'ScrollBar') -or $orientations.ContainsKey($bar.orientation)){throw 'Ambiguous native scrollbar orientation or identity'}
   $orientations[$bar.orientation]=$true
   if($bar.offscreen){continue}
   if($null -eq $bar.bounds){throw 'Visible native scrollbar has no bounds'}
   $b=$bar.bounds;$o=$owner.bounds
   foreach($edge in @($b.Left,$b.Top,$b.Right,$b.Bottom,$o.Left,$o.Top,$o.Right,$o.Bottom)){if([double]::IsNaN($edge) -or [double]::IsInfinity($edge)){throw 'Nonfinite scrollbar or owner bounds'}}
   if($b.Right -le $b.Left -or $b.Bottom -le $b.Top -or $b.Left -lt $o.Left -or $b.Top -lt $o.Top -or $b.Right -gt $o.Right -or $b.Bottom -gt $o.Bottom){throw 'Scrollbar bounds are not contained in their native owner'}
   ,$b
  }
  if($owner.horizontal -and !$orientations.ContainsKey('Horizontal')){throw 'Scrollable ancestor has no horizontal scrollbar peer'}
  if($owner.vertical -and !$orientations.ContainsKey('Vertical')){throw 'Scrollable ancestor has no vertical scrollbar peer'}
 }
 function Get-SourceChromeGeometry($strip){
  $ancestors=@();$seen=@{};$peer=$strip;$ended=$false
  for($depth=0;$null -ne $peer -and $depth -lt 24;$depth++){
   if($peer.Current.ProcessId -ne $process.Id){$ended=$true;break}
   $id=$peer.Current.AutomationId
   if($id -like 'pax-node-*' -and $peer.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsScrollPatternAvailableProperty)){
    if($seen.ContainsKey($id)){throw 'Duplicate native scroller ancestor identity'};$seen[$id]=$true;$ancestors+=,$peer
   }
   if($peer.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window){$ended=$true;break}
   $peer=[System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($peer)
   if($null -eq $peer){$ended=$true}
  }
  $nativeOuter=Source-Native-Scroller $outer
  if(!$ended -or $null -eq $nativeOuter -or !$seen.ContainsKey($strip.Current.AutomationId) -or !$seen.ContainsKey($nativeOuter.Current.AutomationId)){throw 'Source native scrollbar ancestor scope is incomplete'}
  $observations=@();$exclusions=@()
  $barCondition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::ScrollBar)
  foreach($ancestor in $ancestors){
   $b=$ancestor.Current.BoundingRectangle
   if($b.IsEmpty){throw 'Source native scroller ancestor is offscreen'}
   $pattern=$ancestor.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
   $owner=@{id=$ancestor.Current.AutomationId;bounds=@{Left=$b.Left;Top=$b.Top;Right=$b.Right;Bottom=$b.Bottom};horizontal=$pattern.Current.HorizontallyScrollable;vertical=$pattern.Current.VerticallyScrollable}
   $cache=New-Object System.Windows.Automation.CacheRequest
   foreach($property in @('AutomationIdProperty','BoundingRectangleProperty','IsOffscreenProperty','OrientationProperty','IsEnabledProperty','ProcessIdProperty')){$cache.Add([System.Windows.Automation.AutomationElement]::$property)}
   $cache.TreeScope=[System.Windows.Automation.TreeScope]::Element;$active=$cache.Activate()
   # ScrollView exposes its own scrollbar peers as direct native children.
   # Do not enumerate the page's descendant text or unrelated nested scrollers.
   try{$items=$ancestor.FindAll([System.Windows.Automation.TreeScope]::Children,$barCondition)}finally{$active.Dispose()}
   if($items.Count -gt 2){throw 'Ambiguous direct native scrollbar inventory'}
   $bars=@()
   foreach($bar in $items){
    if($bar.Cached.ProcessId -ne $process.Id){throw 'Foreign scrollbar peer'}
    $barOwner=Source-Native-Scroller $bar;$bb=$bar.Cached.BoundingRectangle
    $bars+=@{id=$bar.Cached.AutomationId;ownerId=if($barOwner){$barOwner.Current.AutomationId}else{''};orientation=$bar.Cached.Orientation.ToString();offscreen=$bar.Cached.IsOffscreen;enabled=$bar.Cached.IsEnabled;bounds=if($bb.IsEmpty){$null}else{@{Left=$bb.Left;Top=$bb.Top;Right=$bb.Right;Bottom=$bb.Bottom}}}
   }
   $exclusions+=@(Select-SourceChromeExclusions $owner $bars);$observations+=@{owner=$owner;bars=$bars}
  }
  return @{utc=[DateTime]::UtcNow.ToString('o');ancestors=$observations;exclusions=$exclusions}
 }
 function Source-Click-State($label,$strip,$names,[int]$x,[int]$y){
  $state=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');point=@{x=$x;y=$y}}
  try{$state.labelBounds=$label.Current.BoundingRectangle.ToString();$state.labelRuntimeId=$label.GetRuntimeId() -join '-';$state.stripBounds=$strip.Current.BoundingRectangle.ToString();$state.viewport=(Visible-Scroll-Bounds $strip).ToString();$state.intersection=(Visible-Scroll-Bounds $label).ToString();$state.scrollPercent=$strip.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern).Current.HorizontalScrollPercent}catch{$state.geometryError=$_.Exception.Message}
  # The current viewer has no separate selected-file metadata label. Selection
  # is proved by the exact document hash after the physical tab click below.
  $state.selectedMetadata=@();$state.selectedMetadataScope='No separate selected-file label is authored'
  $state.hitAncestry=@()
  try{
   $hit=[System.Windows.Automation.AutomationElement]::FromPoint([Windows.Point]::new($x,$y))
   for($depth=0;$null -ne $hit -and $depth -lt 8;$depth++){
    $current=$hit.Current;$hitName=$current.Name;if($hitName.Length -gt 160){$hitName=$hitName.Substring(0,160)}
    $state.hitAncestry+=@{name=$hitName;automationId=$current.AutomationId;controlType=$current.ControlType.ProgrammaticName;bounds=$current.BoundingRectangle.ToString();processId=$current.ProcessId;runtimeId=($hit.GetRuntimeId() -join '-')}
    if($current.ProcessId -ne $process.Id){break}
    $hit=[System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($hit)
   }
  }catch{$state.hitError=$_.Exception.Message}
  return $state
 }
 function Source-Interior-Diagnostic($strip,$codeScroll,[string]$name,$names,$expected){
  $diagnostic=[ordered]@{diagnosticOnly=$true;acceptanceResultUnchanged=$true;label=$name;expectedSha256=$expected.displaySha256Utf8;reveal=@();clicked=$false;matchedExpected=$false;observed=@()}
  try{
   $pattern=$strip.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
   # At most six physical wheel ticks, then one click. Never invoke a source
   # handler or set ScrollPattern offsets to manufacture a successful selection.
   for($step=0;$step -le 6;$step++){
    $rows=@(Tab-Snapshot $strip|Where-Object{$_.name -ceq $name})
    if($rows.Count -ne 1){throw 'Diagnostic source label identity is ambiguous'}
    $item=$rows[0].item;$b=Visible-Scroll-Bounds $item;$viewport=Visible-Scroll-Bounds $strip
    if($viewport.IsEmpty){throw 'Diagnostic source viewport is empty'}
    $position=$pattern.Current.HorizontalScrollPercent
    if(!$b.IsEmpty){
     $point=Get-IntegerClickPoint $b
     $x=if($point){$point.x}else{0};$y=if($point){$point.y}else{0}
     $interior=$null -ne $point -and $b.Width -ge [Math]::Min(80,$rows[0].bounds.Width) -and $b.Height -ge 3 -and $x -ge $viewport.Left+32 -and $x -le $viewport.Right-32 -and $y -ge $viewport.Top+3 -and $y -le $viewport.Bottom-3
     if($interior){
      $diagnostic.before=Source-Click-State $item $strip $names $x $y
      Point $x $y $true;$diagnostic.clicked=$true;Start-Sleep -Milliseconds 500
      for($attempt=0;$attempt -lt 8 -and !$diagnostic.matchedExpected;$attempt++){
       Start-Sleep -Milliseconds 250
       foreach($textPattern in @(Source-Body-Patterns $codeScroll)){
        $content=$textPattern.DocumentRange.GetText(-1)
        if($content.Length -le 80 -or !$content.Contains("`n")){continue}
        $bytes=[Text.Encoding]::UTF8.GetBytes($content.Replace("`r`n","`n"));$hash=[BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($bytes)).Replace('-','').ToLowerInvariant()
        $diagnostic.observed+=@{characters=$content.Length;sha256=$hash}
        if($hash -ceq $expected.displaySha256Utf8){$diagnostic.matchedExpected=$true;break}
       }
      }
      $diagnostic.after=Source-Click-State $item $strip $names $x $y
      break
     }
    }
    if($step -eq 6){break}
    $direction=if($b.IsEmpty){if($position -ge 99.95){-1}else{1}}elseif($b.X+$b.Width/2 -lt $viewport.X+$viewport.Width/2){-1}else{1}
    Wheel-At $strip $direction $true 1
    $after=$pattern.Current.HorizontalScrollPercent
    $diagnostic.reveal+=@{step=$step;direction=$direction;before=$position;after=$after;labelBounds=$rows[0].bounds.ToString();intersection=$b.ToString();viewport=$viewport.ToString()}
    if([Math]::Abs($after-$position) -le .001){break}
   }
   if(!$diagnostic.clicked){$diagnostic.outcome='No interior target reached within six physical wheel ticks'}
  }catch{$diagnostic.error=$_.Exception.Message}
  return $diagnostic
 }
 function Wheel-At($scroll,[int]$direction,[bool]$horizontal=$false,[int]$ticks=3,[double]$fractionX=-1,[double]$fractionY=.5){
  Focus;$b=Visible-Scroll-Bounds $scroll;if($b.IsEmpty){throw 'Scroll target is not visible'}
  # The outer page's margin avoids entering embedded preview scrollers.
  $fraction=if((Scroll-Id $scroll) -eq (Scroll-Id $outer)){.02}else{.5}
  if($fractionX -ge 0){$fraction=$fractionX};Point ([int]($b.X+$b.Width*$fraction)) ([int]($b.Y+$b.Height*$fractionY))
  $flag=if($horizontal){0x1000}else{0x800}
  for($t=0;$t -lt $ticks;$t++){if([PaxControlsInput]::Mouse($flag,$direction*120) -ne 1){throw 'Incomplete wheel input'};Start-Sleep -Milliseconds 60}
  Start-Sleep -Milliseconds 500;[void](Website-Sample 'interaction');Save
 }
 function Get-VisibleViewportPoint($bounds,$viewport){
  $intersection=[pscustomobject]@{Left=[Math]::Max($bounds.Left,$viewport.Left);Top=[Math]::Max($bounds.Top,$viewport.Top);Right=[Math]::Min($bounds.Right,$viewport.Right);Bottom=[Math]::Min($bounds.Bottom,$viewport.Bottom)}
  return Get-IntegerClickPoint $intersection
 }
 function Reveal($item,$scroll,[bool]$horizontal=$false,[double]$targetFraction=0,[bool]$fullViewport=$false){
  $pattern=$scroll.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
  $initial=if($horizontal){$pattern.Current.HorizontalScrollPercent}else{$pattern.Current.VerticalScrollPercent}
  $searchDirection=if($horizontal){1}else{if($initial -ge 50){1}else{-1}}
  for($attempt=0;$attempt -lt 90;$attempt++){
   $b=$item.Current.BoundingRectangle;$viewport=Visible-Scroll-Bounds $scroll
   if($viewport.IsEmpty){throw 'Reveal viewport is empty'}
   if($b.IsEmpty){
    # UIA may intentionally omit bounds for clipped offscreen text. Search the
    # real native viewport instead of interpreting Empty's infinities as layout.
    $position=if($horizontal){$pattern.Current.HorizontalScrollPercent}else{$pattern.Current.VerticalScrollPercent}
    if($position -ge 99.95){$searchDirection=if($horizontal){-1}else{1}}
    if($position -le .05){$searchDirection=if($horizontal){1}else{-1}}
    $report.websiteRevealSamples+=@{name=$item.Current.Name;bounds='Empty';viewport=$viewport.ToString();direction=$searchDirection;horizontal=$horizontal;outerY=$outerPattern.Current.VerticalScrollPercent;searchPosition=$position}
    Wheel-At $scroll $searchDirection $horizontal 4;continue
   }
   # A footer can remain below the usual upper targeting band at maximum scroll.
   # Only callers explicitly opting in may accept its full visible viewport.
   if(!$horizontal -and $fullViewport){
    $visible=Visible-Scroll-Bounds $item
    if(!$visible.IsEmpty -and $null -ne (Get-VisibleViewportPoint $visible $viewport)){return}
   }
   # A long tab need only expose a real clickable slice; insisting that its
   # unclipped left edge enter the viewport can cause repeated end-to-end scans.
   if($horizontal){$visible=Visible-Scroll-Bounds $item;if(!$visible.IsEmpty -and $visible.Width -ge [Math]::Min(20,$b.Width) -and $visible.Height -ge 3){return}}
   if($horizontal){$center=$b.X+[Math]::Min($b.Width,80)/2;$low=$viewport.Left+10;$high=$viewport.Right-10}
   # The website's fixed 80px navigation can cover otherwise visible native
   # bounds. Reveal ordinary page targets below that authored overlay.
   else{$center=$b.Y+$b.Height/2;$low=$viewport.Top+120*$scale;$high=$viewport.Top+[Math]::Min(400*$scale,$viewport.Height-30)}
   if(!$horizontal -and $targetFraction -gt 0){$low=$viewport.Top+$viewport.Height*$targetFraction-65;$high=$low+130}
   if($center -ge $low -and $center -le $high -and (Visible-Item $item)){return}
   $direction=if($center -lt $low){if($horizontal){-1}else{1}}else{if($horizontal){1}else{-1}}
   $report.websiteRevealSamples+=@{name=$item.Current.Name;bounds=$b.ToString();viewport=$viewport.ToString();direction=$direction;horizontal=$horizontal;outerY=$outerPattern.Current.VerticalScrollPercent}
   Wheel-At $scroll $direction $horizontal 1
  }
  throw ('Could not reveal authored item '+$item.Current.Name)
 }
 function Assert-Visible([string]$name){$found=@(Visible-Named $name);Check "Visible native text: $name" ($found.Count -gt 0) $found.Count;return $found}
 function Click-Text([string]$name,[bool]$lowest=$false){
  $found=@(Visible-Named $name);Check "Input target exists: $name" ($found.Count -gt 0) $found.Count
  $item=if($lowest){$found|Sort-Object {$_.Current.BoundingRectangle.Top} -Descending|Select-Object -First 1}else{$found|Sort-Object {$_.Current.BoundingRectangle.Width} -Descending|Select-Object -First 1}
  Click-Visible $item
 }
 function Mark([string]$name){$report.websiteInteractionSteps+=@{name=$name;utc=[DateTime]::UtcNow.ToString('o');sample=(Website-Sample $name)};Shot $name;Save}

 if($WebsiteHeaderOnly){
  . (Join-Path $PSScriptRoot 'website-header-acceptance.ps1')
  Invoke-WebsiteHeaderAcceptance
  return
 }

 if(!$WebsiteSourcesOnly -and !$WebsiteRouteOnly){
 # The homepage now authors screenshot links; its live calculator is mounted
 # by /calculator/. Exercise that actual route before the unchanged keypad and
 # native graph assertions, then return through its authored Back to Pax link.
 $calculatorLink=Named 'TRY THE CALCULATOR  ↗'|Select-Object -First 1
 Check 'Calculator authored live-route link exists' ($null -ne $calculatorLink) '/calculator/'
 Reveal $calculatorLink $outer;Click-Visible $calculatorLink;Start-Sleep -Seconds 2
 [void](Assert-Visible 'CALCULATOR / LIVE')
 $calc=Named 'CALCULATE'|Select-Object -First 1;Check 'Calculator route mounts actual keypad' ($null -ne $calc) 'CALCULATE'
 $calculatorRoot=Source-Native-Scroller $calc
 Check 'Calculator has its own native scroll container' ($null -ne $calculatorRoot) $(if($calculatorRoot){Scroll-Id $calculatorRoot}else{'absent'})
 $calculatorRootId=Scroll-Id $calculatorRoot
 $outer=$calculatorRoot;$outerPattern=$outer.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
 # The route fills its viewport; its keypad need not enter an upper-page band.
 Reveal $calc $outer $false 0 $true
 function Graph-State {
  $rows=@(Text-Snapshot $calculatorRoot|ForEach-Object{$_.name})
  $range=@($rows|Where-Object{$_ -match '^x [-0-9.]+'})|Select-Object -First 1
  Check 'Graph exposes numeric range' ($range -match '^x ([-0-9.]+) … ([-0-9.]+)   y ([-0-9.]+) … ([-0-9.]+)$') $range
  $data=@{text=$range;xMin=[double]$Matches[1];xMax=[double]$Matches[2];yMin=[double]$Matches[3];yMax=[double]$Matches[4]}
  Check 'Graph has resolved curve away from world edge' (!($rows -contains 'NO RESOLVED CURVE') -and !($rows -contains 'WORLD EDGE')) @($rows|Where-Object{$_ -in @('NO RESOLVED CURVE','WORLD EDGE')})
  return $data
 }
 $initialGraph=Graph-State
 Check 'Default graph range is centered' ([Math]::Abs($initialGraph.xMin+$initialGraph.xMax) -le .2 -and [Math]::Abs($initialGraph.yMin+$initialGraph.yMax) -le .2) $initialGraph
 Mark 'calculator-default-graph'
 Click-Text 'CALCULATE';[void](Assert-Visible 'CALCULATE  /  RAD')
 foreach($key in @('2','+','3','enter')){Click-Text $key $true}
 $rows=@(Text-Snapshot $calculatorRoot|Where-Object{$_.name -ceq '› 2+3' -or $_.name.Trim() -ceq '5'}|Where-Object{Visible-Item $_.item}|ForEach-Object{$_.name})
 Check 'Calculator evaluates actual keypad expression' (($rows -contains '› 2+3') -and @($rows|Where-Object{$_.Trim() -ceq '5' -and $_.Length -gt 1}).Count -gt 0) @($rows|Where-Object{$_ -like '*2+3*' -or $_.Trim() -ceq '5'})
 Mark 'calculator-evaluated';Click-Text 'GRAPH';[void](Assert-Visible 'POLAR  /  RAD  /  0…2π');Start-Sleep -Seconds 1
 $restoredGraph=Graph-State
 Check 'Restored graph range stays centered' ([Math]::Abs($restoredGraph.xMin+$restoredGraph.xMax) -le .2 -and [Math]::Abs($restoredGraph.yMin+$restoredGraph.yMax) -le .2) $restoredGraph
 $zoom=@(Visible-Named '+')|Sort-Object {$_.Current.BoundingRectangle.Top}|Select-Object -First 1;Click-Visible $zoom;Start-Sleep -Seconds 1
 $zoomedGraph=Graph-State
 Check 'Graph zoom preserves center and narrows range' ([Math]::Abs($zoomedGraph.xMin+$zoomedGraph.xMax) -le .2 -and [Math]::Abs($zoomedGraph.yMin+$zoomedGraph.yMax) -le .2 -and ($zoomedGraph.xMax-$zoomedGraph.xMin) -lt ($restoredGraph.xMax-$restoredGraph.xMin)) @{before=$restoredGraph;after=$zoomedGraph}
 $graphScroll=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)|Where-Object{(Visible-Item $_) -and $_.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern).Current.HorizontallyScrollable -and $_.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern).Current.VerticallyScrollable})|Sort-Object {$_.Current.BoundingRectangle.Width}|Select-Object -First 1
 Check 'Graph native two-axis scroller exists' ($null -ne $graphScroll) 'actual ScrollPattern'
 $graphPattern=$graphScroll.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern);$outerBeforeGraph=$outerPattern.Current.VerticalScrollPercent
 Check 'Graph native offsets are centered' ([Math]::Abs($graphPattern.Current.HorizontalScrollPercent-50) -lt .1 -and [Math]::Abs($graphPattern.Current.VerticalScrollPercent-50) -lt .1) @{x=$graphPattern.Current.HorizontalScrollPercent;y=$graphPattern.Current.VerticalScrollPercent}
 foreach($location in @(@{name='center';x=.5;y=.5},@{name='blank';x=.12;y=.18})){
  foreach($axis in @('horizontal','vertical')){
   $horizontal=$axis -eq 'horizontal';$before=if($horizontal){$graphPattern.Current.HorizontalScrollPercent}else{$graphPattern.Current.VerticalScrollPercent}
   $bounds=Visible-Scroll-Bounds $graphScroll;$point=[Windows.Point]::new($bounds.X+$bounds.Width*$location.x,$bounds.Y+$bounds.Height*$location.y)
   $hit=[System.Windows.Automation.AutomationElement]::FromPoint($point);$ancestry=@();$peer=$hit
   for($depth=0;$depth -lt 12 -and $peer -and $peer.Current.ProcessId -eq $process.Id;$depth++){$ancestry+=@{name=$peer.Current.Name;automationId=$peer.Current.AutomationId;type=$peer.Current.ControlType.ProgrammaticName};$peer=[System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($peer)}
   $outerBeforeWheel=$outerPattern.Current.VerticalScrollPercent
   Wheel-At $graphScroll $(if($horizontal){1}else{-1}) $horizontal 2 $location.x $location.y
   $samples=@();for($i=0;$i -lt 10;$i++){$samples+=@{x=$graphPattern.Current.HorizontalScrollPercent;y=$graphPattern.Current.VerticalScrollPercent;outerY=$outerPattern.Current.VerticalScrollPercent};Start-Sleep -Milliseconds 100}
   $after=if($horizontal){$graphPattern.Current.HorizontalScrollPercent}else{$graphPattern.Current.VerticalScrollPercent}
   Observe "Graph $($location.name) real $axis wheel pans native viewport" ([Math]::Abs($after-$before) -gt .01) @{before=$before;after=$after;bounds=$bounds.ToString();input=$point.ToString();hitAncestry=$ancestry;samples=$samples}
   Observe "Graph $($location.name) $axis pan retains outer page position" ([Math]::Abs($outerPattern.Current.VerticalScrollPercent-$outerBeforeWheel) -lt .05) @{before=$outerBeforeWheel;after=$outerPattern.Current.VerticalScrollPercent}
  }
 }
 $outerBeforeDrag=$outerPattern.Current.VerticalScrollPercent;$beforeDrag=@{x=$graphPattern.Current.HorizontalScrollPercent;y=$graphPattern.Current.VerticalScrollPercent};$bounds=Visible-Scroll-Bounds $graphScroll
 Point ([int]($bounds.X+$bounds.Width*.3)) ([int]($bounds.Y+$bounds.Height*.3))
 if([PaxControlsInput]::Mouse(2) -ne 1){throw 'Graph drag press failed'}
 try{for($i=1;$i -le 5;$i++){Point ([int]($bounds.X+$bounds.Width*.3+$i*8)) ([int]($bounds.Y+$bounds.Height*.3+$i*4))}}finally{if([PaxControlsInput]::Mouse(4) -ne 1){throw 'Graph drag release failed'}}
 Start-Sleep -Seconds 1;$afterDrag=@{x=$graphPattern.Current.HorizontalScrollPercent;y=$graphPattern.Current.VerticalScrollPercent}
 Observe 'Graph authored drag changes both native offsets' ([Math]::Abs($afterDrag.x-$beforeDrag.x) -gt .005 -and [Math]::Abs($afterDrag.y-$beforeDrag.y) -gt .005) @{before=$beforeDrag;after=$afterDrag}
 Observe 'Graph drag retains outer page position' ([Math]::Abs($outerPattern.Current.VerticalScrollPercent-$outerBeforeDrag) -lt .05) @{before=$outerBeforeDrag;after=$outerPattern.Current.VerticalScrollPercent}
 Mark 'calculator-graph-zoom-pan'
 Click-Text '← BACK TO PAX';Start-Sleep -Seconds 2
 Check 'Calculator demo route retires after return' (@(Visible-Named 'CALCULATOR / LIVE').Count -eq 0) 'route title absent'
 $homeScrolls=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
 $outer=@($homeScrolls|Where-Object{Visible-Item $_}|Sort-Object @{Expression={$b=Visible-Scroll-Bounds $_;$b.Width*$b.Height};Descending=$true})|Select-Object -First 1
 Check 'Calculator Back to Pax remounts home' ($null -ne $outer -and (Scroll-Id $outer) -ne $calculatorRootId) 'new native home scroller'
 $outerPattern=$outer.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)

 $launch=Named 'OPEN PAXFLIX FULL SCREEN  ↗'|Select-Object -First 1;if($null -eq $launch){$launch=Named 'TRY PAXFLIX  ↗'|Select-Object -First 1}
 Check 'Paxflix authored full-screen launch exists' ($null -ne $launch) 'Open Paxflix full screen';Reveal $launch $outer;Click-Visible $launch;Start-Sleep -Seconds 2
 [void](Assert-Visible 'PAXFLIX / LIVE');Mark 'paxflix-open'
 Click-Text 'Play film';[void](Assert-Visible 'Paxflix / FEATURE');Click-Text 'Play';[void](Assert-Visible 'You found your next watch. Playback is a demo.')
 Click-Text '↓   Download';[void](Assert-Visible 'Saved for an imaginary journey. Download is a demo.');Mark 'paxflix-detail-feedback';Click-Text '×'
 $profile=Visible-Named 'P'|Sort-Object {$_.Current.BoundingRectangle.Right} -Descending|Select-Object -First 1;Check 'Paxflix profile input exists' ($null -ne $profile) 'P';Click-Visible $profile
 [void](Assert-Visible 'YOUR Paxflix');Click-Text 'Dark mode';[void](Assert-Visible 'Light mode');Mark 'paxflix-light-theme'
 Click-Text 'Manage profiles';[void](Assert-Visible "You're watching as the demo profile.");Click-Text 'Light mode';[void](Assert-Visible 'Dark mode');Click-Text '×'
 Click-Text '← BACK TO PAX';Start-Sleep -Seconds 2
 Check 'Paxflix route retires after return' (@(Visible-Named 'PAXFLIX / LIVE').Count -eq 0) 'route title absent'
 $homeScrolls=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
 $outer=@($homeScrolls|Where-Object{Visible-Item $_}|Sort-Object @{Expression={$b=Visible-Scroll-Bounds $_;$b.Width*$b.Height};Descending=$true})|Select-Object -First 1
 Check 'Paxflix Back to Pax remounts home' ($null -ne $outer) 'new native home scroller'
 $outerPattern=$outer.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern);Mark 'paxflix-returned-home'

 # Gallery hover pauses its authored marquee; real horizontal wheel must move
 # that native scroller while retaining the outer vertical position.
 $gallery=Named 'Meet your creative toolkit'|Select-Object -First 1;Check 'Current authored gallery heading exists' ($null -ne $gallery) 'Meet your creative toolkit';Reveal $gallery $outer
 Wheel-At $outer -1 $false 3
 $allScrolls=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
 $galleryScroll=@($allScrolls|Where-Object{(Visible-Item $_) -and $_.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern).Current.HorizontallyScrollable}|Sort-Object @{Expression={(Visible-Scroll-Bounds $_).Width};Descending=$true})|Select-Object -First 1
 Check 'Gallery horizontal native scroller visible' ($null -ne $galleryScroll) 'actual native ScrollPattern'
 $gp=$galleryScroll.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern);$gb=Visible-Scroll-Bounds $galleryScroll;Point ([int]($gb.X+$gb.Width*.5)) ([int]($gb.Y+$gb.Height*.5));Start-Sleep -Seconds 1
 $beforeX=$gp.Current.HorizontalScrollPercent;$beforeY=$outerPattern.Current.VerticalScrollPercent;Wheel-At $galleryScroll 1 $true 3
 Check 'Gallery real horizontal wheel moves content' ([Math]::Abs($gp.Current.HorizontalScrollPercent-$beforeX) -gt .01) @{before=$beforeX;after=$gp.Current.HorizontalScrollPercent}
 Check 'Gallery horizontal wheel retains outer Y' ([Math]::Abs($outerPattern.Current.VerticalScrollPercent-$beforeY) -lt .05) @{before=$beforeY;after=$outerPattern.Current.VerticalScrollPercent};Mark 'gallery-wheel'

 }

 if($WebsiteDemosOnly){$report.websiteDemoAcceptance=($report.websiteInteractionIssues.Count -eq 0);Mark 'website-demos-complete';if($report.websiteInteractionIssues.Count){throw ($report.websiteInteractionIssues -join '; ')};return}

 if(!$WebsiteRouteOnly){
 # Follow the authored section order recorded beside the source manifest.
 # Empty offscreen UIA bounds have no ordering information. The toggle also
 # unmounts while its drawer is open, so RuntimeID is only an observation receipt.
 $sectionPath=[IO.Path]::ChangeExtension($WebsiteSourceManifest,'sections.json')
 Check 'Authored section manifest exists' (Test-Path -LiteralPath $sectionPath) $sectionPath
 $sectionManifest=Get-Content -LiteralPath $sectionPath -Raw|ConvertFrom-Json
 Check 'Section manifest schema is supported' ($sectionManifest.schemaVersion -eq 1) $sectionManifest.schemaVersion
 $sourceHash=(Get-FileHash -LiteralPath $WebsiteSourceManifest -Algorithm SHA256).Hash.ToLowerInvariant()
 Check 'Section inventory is bound to exact source manifest' ($sectionManifest.sourceManifestSha256 -ceq $sourceHash) $sourceHash
 $drawerPlan=@($sectionManifest.sections)
 Check 'Section inventory has six unique authored sections' ($drawerPlan.Count -eq 6 -and @($drawerPlan.id|Sort-Object -Unique).Count -eq 6) @($drawerPlan.id)
 $plannedOccurrences=@($drawerPlan|ForEach-Object{$_.sources}|ForEach-Object{$_.label+'|'+$_.displaySha256Utf8}|Sort-Object)
 $frozenOccurrences=@($sourceRows|ForEach-Object{$_.label+'|'+$_.displaySha256Utf8}|Sort-Object)
 Check 'Section inventory preserves all 102 frozen source occurrences' ($sectionManifest.expectedSourceOccurrences -eq 102 -and $frozenOccurrences.Count -eq 102 -and ($plannedOccurrences -join "`n") -ceq ($frozenOccurrences -join "`n")) @{expected=$frozenOccurrences.Count;actual=$plannedOccurrences.Count}
 Check 'Source viewer matches current authored native controls' ($sectionManifest.sourceViewer.toggleName -ceq 'VIEW SOURCE' -and $sectionManifest.sourceViewer.toggleControlType -ceq 'Button' -and $sectionManifest.sourceViewer.closeName -ceq '×' -and $sectionManifest.sourceViewer.tabViewportHeight -eq 48 -and $sectionManifest.sourceViewer.tabHeight -eq 36) $sectionManifest.sourceViewer
 $report.websiteSourceTraversalProvenance=@{sectionManifestSha256=(Get-FileHash -LiteralPath $sectionPath -Algorithm SHA256).Hash.ToLowerInvariant();sourceManifestSha256=$sourceHash;sourceProvenance=$sectionManifest.sourceProvenance;expectedSectionCounts=@($drawerPlan|ForEach-Object{@($_.sources).Count})}
 # Source discovery below uses the website's wide layout, where every section
 # kicker is adjacent to its drawer. Keep this probe above compact breakpoints.
 Check 'Source drawer qualification uses the wide website layout' ($report.clientViewport.width -ge 1000) $report.clientViewport
 $report.websiteSourceDrawerObservations=@();$report.websiteSourceSections=@();Save
 $sourceButtons=@(Named 'VIEW SOURCE' ([System.Windows.Automation.ControlType]::Button))
 Check 'All six authored source drawers exist' ($sourceButtons.Count -eq 6) $sourceButtons.Count
 function Finite-Source-Bounds($b){return !$b.IsEmpty -and $b.Width -gt 0 -and $b.Height -gt 0 -and ![double]::IsInfinity($b.Top) -and ![double]::IsInfinity($b.Left) -and ![double]::IsNaN($b.Top) -and ![double]::IsNaN($b.Left)}
 # Only native wheel input moves the page. Rewind once, then search forward for
 # each known next section; do not sweep to the opposite end for Empty bounds.
 for($rewind=0;$rewind -lt 90 -and !(Website-End 1);$rewind++){Wheel-At $outer 1 $false 8}
 Check 'Source traversal starts at native page top' (Website-End 1) $outerPattern.Current.VerticalScrollPercent
 function Test-SourceSectionReady($anchor,$toggle,$viewport,[double]$navHeight,[int]$candidateCount,[string]$expectedId,[string]$actualId){
  if($candidateCount -ne 1 -or [string]::IsNullOrWhiteSpace($expectedId) -or $actualId -cne $expectedId){return $false}
  if([double]::IsNaN($navHeight) -or [double]::IsInfinity($navHeight) -or $navHeight -lt 0){return $false}
  foreach($b in @($anchor,$toggle,$viewport)){
   foreach($edge in @($b.Left,$b.Top,$b.Right,$b.Bottom)){if([double]::IsNaN($edge) -or [double]::IsInfinity($edge)){return $false}}
   if($b.Right -le $b.Left -or $b.Bottom -le $b.Top){return $false}
  }
  foreach($b in @($anchor,$toggle)){
   if($b.Left -lt $viewport.Left -or $b.Right -gt $viewport.Right -or $b.Top -lt $viewport.Top+$navHeight -or $b.Bottom -gt $viewport.Bottom -or $null -eq (Get-IntegerClickPoint $b)){return $false}
  }
  return $true
 }
 function Source-Drawer-Toggle($drawer){
  $anchor=$null;$anchorBounds=$null
  if($null -ne $drawer.anchor){
   $anchors=@(Named $drawer.anchor)
   Check 'Authored source section anchor is unique' ($anchors.Count -eq 1) @{section=$drawer.id;anchor=$drawer.anchor;count=$anchors.Count}
   $anchor=$anchors[0];$stalled=0;$found=$false
   for($advance=0;$advance -lt 120;$advance++){
    $b=$anchor.Current.BoundingRectangle
    if(Finite-Source-Bounds $b){$found=$true;break}
    $before=$outerPattern.Current.VerticalScrollPercent
    Check 'Next authored source anchor can be reached forward' ($before -lt 99.95 -and $stalled -lt 3) @{section=$drawer.id;position=$before;stalled=$stalled}
    Wheel-At $outer -1 $false 3
    $after=$outerPattern.Current.VerticalScrollPercent
    $stalled=if($after -le $before+.001){$stalled+1}else{0}
    $report.websiteRevealSamples+=@{kind='source-section-forward';section=$drawer.id;anchor=$drawer.anchor;before=$before;after=$after;stalled=$stalled};Save
   }
   Check 'Next source section has real observed bounds' $found @{section=$drawer.id;anchor=$drawer.anchor}
   # Use ordinary page navigation, not a narrow centering band that a native
   # wheel notch can skip. Actual readiness is checked against nav/viewport below.
   Reveal $anchor $outer
   $anchorBounds=Visible-Scroll-Bounds $anchor
   Check 'Source section anchor is visibly revealed' (Finite-Source-Bounds $anchorBounds) @{section=$drawer.id;bounds=$anchorBounds.ToString()}
   # All five non-hero kickers are authored at section y=80. ExampleHost's
   # toggle is at y=24; allow text metric differences, but require one candidate.
   $visible=@(Visible-Named 'VIEW SOURCE' ([System.Windows.Automation.ControlType]::Button)|Where-Object{$b=$_.Current.BoundingRectangle;(Finite-Source-Bounds $b) -and $b.Top -ge $anchorBounds.Top-120 -and $b.Top -le $anchorBounds.Top+20})
   Check 'Visible source toggle belongs unambiguously to authored anchor' ($visible.Count -eq 1) @{section=$drawer.id;anchor=$anchorBounds.ToString();candidates=@($visible|ForEach-Object{$_.Current.BoundingRectangle.ToString()})}
   $toggle=$visible[0];$expectedToggleId=$toggle.GetRuntimeId() -join '-'
   Reveal $toggle $outer
   $anchorBounds=Visible-Scroll-Bounds $anchor;$sourceViewport=Visible-Scroll-Bounds $outer
   $fresh=@(Visible-Named 'VIEW SOURCE' ([System.Windows.Automation.ControlType]::Button)|Where-Object{$b=$_.Current.BoundingRectangle;(Finite-Source-Bounds $b) -and $b.Top -ge $anchorBounds.Top-120 -and $b.Top -le $anchorBounds.Top+20})
   $actualToggleId=if($fresh.Count -eq 1){$fresh[0].GetRuntimeId() -join '-'}else{''}
   $toggleBounds=if($fresh.Count -eq 1){$fresh[0].Current.BoundingRectangle}else{[Windows.Rect]::Empty}
   $ready=Test-SourceSectionReady $anchorBounds $toggleBounds $sourceViewport (80*$scale) $fresh.Count $expectedToggleId $actualToggleId
   Check 'Source anchor and unique toggle are ready below navigation' $ready @{section=$drawer.id;anchor=$anchorBounds.ToString();toggle=$toggleBounds.ToString();viewport=$sourceViewport.ToString();navHeight=80*$scale;candidates=$fresh.Count;expectedRuntimeId=$expectedToggleId;actualRuntimeId=$actualToggleId}
   $toggle=$fresh[0]
  }else{
   Check 'Hero source drawer is located from native top' (Website-End 1) $outerPattern.Current.VerticalScrollPercent
   $visible=@(Visible-Named 'VIEW SOURCE' ([System.Windows.Automation.ControlType]::Button)|Where-Object{Finite-Source-Bounds $_.Current.BoundingRectangle}|Sort-Object {$_.Current.BoundingRectangle.Top})
   Check 'Hero source toggle is visible at page top' ($visible.Count -gt 0) $visible.Count
   $toggle=$visible[0]
  }
  $report.websiteSourceDrawerObservations+=@{section=$drawer.id;utc=[DateTime]::UtcNow.ToString('o');anchor=$drawer.anchor;anchorBounds=if($anchorBounds){$anchorBounds.ToString()}else{$null};toggleBounds=$toggle.Current.BoundingRectangle.ToString();toggleRuntimeId=($toggle.GetRuntimeId() -join '-');outerY=$outerPattern.Current.VerticalScrollPercent};Save
  return $toggle
 }
 for($section=0;$section -lt $drawerPlan.Count;$section++){
  $drawer=$drawerPlan[$section]
  $toggle=Source-Drawer-Toggle $drawer
  $beforeSourceScrollIds=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)|Where-Object{$_.Current.AutomationId -like 'pax-node-*'}|ForEach-Object{$_.Current.AutomationId})
  Click-Visible $toggle;Start-Sleep -Seconds 1
  $allScrolls=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
  $newTabScrolls=@($allScrolls|Where-Object{$_.Current.AutomationId -like 'pax-node-*' -and $_.Current.AutomationId -notin $beforeSourceScrollIds -and (Visible-Item $_) -and [Math]::Abs($_.Current.BoundingRectangle.Height-48*$scale) -le 2*$scale})
  Check "Section $section source tab strip exists uniquely" ($newTabScrolls.Count -eq 1) @($newTabScrolls|ForEach-Object{@{id=$_.Current.AutomationId;bounds=$_.Current.BoundingRectangle.ToString()}})
  $tabs=$newTabScrolls[0]
  $nativeTabs=Source-Native-Scroller $tabs
  Check 'Source tab peer has an owned native scroller' ($null -ne $nativeTabs) $(if($nativeTabs){$nativeTabs.Current.AutomationId}else{'absent'})
  $nativeTabId=$nativeTabs.Current.AutomationId;$nativeTabBounds=$nativeTabs.Current.BoundingRectangle
  Check 'Opened source tab scroller is newly mounted' ($nativeTabId -notin $beforeSourceScrollIds) @{id=$nativeTabId;before=$beforeSourceScrollIds}
  $codeCandidates=@();$newSourceScrolls=@();$alignmentTolerance=2*$scale
  foreach($candidate in $allScrolls){
   $id=$candidate.Current.AutomationId
   if($id -notlike 'pax-node-*' -or $id -in $beforeSourceScrollIds -or $id -eq $nativeTabId){continue}
   $b=$candidate.Current.BoundingRectangle
   # The authored code viewport starts directly below the tab strip, with
   # the same left/right edges. Groups between them do not form native peers.
   $aligned=(Finite-Source-Bounds $b) -and [Math]::Abs($b.Left-$nativeTabBounds.Left) -le $alignmentTolerance -and [Math]::Abs($b.Right-$nativeTabBounds.Right) -le $alignmentTolerance -and [Math]::Abs($b.Top-$nativeTabBounds.Bottom) -le $alignmentTolerance
   $visible=$aligned -and (Visible-Item $candidate)
   $newSourceScrolls+=@{id=$id;bounds=$b.ToString();aligned=$aligned;visible=$visible}
   if($visible -and $candidate.Current.ProcessId -eq $process.Id){$codeCandidates+=,$candidate}
  }
  Check 'Opened source drawer has one new code scroller below tabs' ($codeCandidates.Count -eq 1) @{tabId=$nativeTabId;tabBounds=$nativeTabBounds.ToString();candidates=$newSourceScrolls}
  $codeScroll=$codeCandidates[0]
  $report.websiteSourceDrawerObservations[-1].codeScroller=@{id=$codeScroll.Current.AutomationId;bounds=$codeScroll.Current.BoundingRectangle.ToString();runtimeId=($codeScroll.GetRuntimeId() -join '-');tabId=$nativeTabId};Save
  $inventory=@(Tab-Snapshot $tabs)
  $expectedNames=@($inventory|ForEach-Object{$_.name})
  Check "Section $section has distinct source tab identities" (($expectedNames.Count -gt 0) -and (@($expectedNames|Sort-Object -Unique).Count -eq $expectedNames.Count)) $expectedNames
  foreach($name in $expectedNames){Check 'Source tab has frozen source identity' ($sourceTruth.ContainsKey($name)) $name}
  $plannedNames=@($drawer.sources|ForEach-Object{$_.label}|Sort-Object)
  $observedNames=@($expectedNames|Sort-Object)
  Check 'Opened source drawer matches exact authored section inventory' (($observedNames -join "`n") -ceq ($plannedNames -join "`n")) @{section=$drawer.id;expected=$plannedNames;observed=$observedNames}
  foreach($expectedRow in $drawer.sources){Check 'Section source hash agrees with frozen manifest' ($sourceTruth[$expectedRow.label].displaySha256Utf8 -ceq $expectedRow.displaySha256Utf8) @{section=$drawer.id;label=$expectedRow.label}}
  $tabPattern=$tabs.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
  for($rewind=0;$rewind -lt 90 -and $tabPattern.Current.HorizontalScrollPercent -gt .05;$rewind++){Wheel-At $tabs -1 $true 1}
  Check "Section $section starts at first source tab" ($tabPattern.Current.HorizontalScrollPercent -le .05) $tabPattern.Current.HorizontalScrollPercent
  $visited=@{};$sourcePattern=$null;$advances=0;$stalled=0
  while($visited.Count -lt $expectedNames.Count){
   $viewport=Visible-Scroll-Bounds $tabs
   Check 'Source tab viewport remains visible' (!$viewport.IsEmpty) $section
   $visible=@(Tab-Snapshot $tabs|Where-Object{!$visited.ContainsKey($_.name) -and (Tab-Is-Visible $_ $viewport)}|Sort-Object {$_.bounds.Left})
   $clickBounds=$null;$clickPoint=$null
   $chrome=$null;$unobscured=$null
   if($visible.Count){
    $clickBounds=Visible-Scroll-Bounds $visible[0].item;$clickPoint=Get-IntegerClickPoint $clickBounds
    if($null -ne $clickPoint){
     # Hover reveals native auto-scrollbar chrome; no click has occurred yet.
     Point $clickPoint.x $clickPoint.y
     $hoverPoint=$clickPoint;$chrome=Get-SourceChromeGeometry $tabs
     $clickBounds=Visible-Scroll-Bounds $visible[0].item;$viewport=Visible-Scroll-Bounds $tabs
     if($clickBounds.IsEmpty -or $viewport.IsEmpty){throw 'Source target moved offscreen during chrome observation'}
     $unobscured=Get-UnobscuredSourcePoint $clickBounds $viewport $chrome.exclusions
     $clickPoint=if($unobscured){$unobscured.point}else{$null}
     $report.websiteSourceChromeHits+=@{section=$section;label=$visible[0].name;hoverPoint=$hoverPoint;labelBounds=$clickBounds.ToString();viewport=$viewport.ToString();chrome=$chrome;unobscured=$unobscured};Save
    }
   }
   if(!$visible.Count -or $null -eq $clickPoint){
    $before=$tabPattern.Current.HorizontalScrollPercent
    if($advances -ge 90 -or $stalled -ge 3 -or $before -ge 99.95){
     $missing=@($expectedNames|Where-Object{!$visited.ContainsKey($_)})
     Check "Section $section can reveal all source tabs monotonically" $false @{missing=$missing;position=$before;advances=$advances;stalled=$stalled}
    }
    Wheel-At $tabs 1 $true 1;$advances++
    $after=$tabPattern.Current.HorizontalScrollPercent
    $stalled=if($after -le $before+.001){$stalled+1}else{0}
    $report.websiteRevealSamples+=@{kind='source-frontier';section=$section;before=$before;after=$after;visited=$visited.Count;expected=$expectedNames.Count};Save
    continue
   }
   # Read one visible tab, then refresh bounds. The native control may reflow
   # after a selection; cached geometry must not survive that input action.
   $label=$visible[0].item;$name=$visible[0].name;$tab=$visited.Count
   Check 'Visible tab belongs to current drawer' ($name -in $expectedNames) $name
   # A one-pixel clipped label still qualifies. Assert containment independently
   # of point construction before dispatching its physical frontier click.
   $clickX=$clickPoint.x;$clickY=$clickPoint.y
   Check 'Source click is inside visible label and viewport' ($clickX -ge $clickBounds.Left -and $clickX -lt $clickBounds.Right -and $clickY -ge $clickBounds.Top -and $clickY -lt $clickBounds.Bottom -and $clickX -ge $viewport.Left -and $clickX -lt $viewport.Right -and $clickY -ge $viewport.Top -and $clickY -lt $viewport.Bottom) @{point=$clickPoint;label=$clickBounds.ToString();viewport=$viewport.ToString()}
   $overlappingBars=@($chrome.exclusions|Where-Object{$clickX -ge $_.Left -and $clickX -lt $_.Right -and $clickY -ge $_.Top -and $clickY -lt $_.Bottom})
   Check 'Source click excludes observed native scrollbar chrome' ($overlappingBars.Count -eq 0 -and $null -ne $unobscured) @{point=$clickPoint;exclusions=$chrome.exclusions;region=$unobscured.region}
   $clickReceipt=[ordered]@{section=$section;tab=$tab;label=$name;kind='acceptance';before=(Source-Click-State $label $tabs $expectedNames $clickX $clickY)}
   $report.websiteSourceClicks+=,$clickReceipt;Save
   Point $clickX $clickY $true;Start-Sleep -Milliseconds 500
   $matched=$false;$seen=@();$expected=$sourceTruth[$name]
   for($attempt=0;$attempt -lt 12 -and !$matched;$attempt++){
    Start-Sleep -Milliseconds 250
    $patterns=@()
    if($null -ne $sourcePattern){$patterns+=,$sourcePattern}
    else{$patterns=@(Source-Body-Patterns $codeScroll)}
    foreach($pattern in $patterns){
      $content=$pattern.DocumentRange.GetText(-1)
      if($content.Length -gt 80 -and $content.Contains("`n")){
       $bytes=[Text.Encoding]::UTF8.GetBytes($content.Replace("`r`n","`n"));$hash=[BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($bytes)).Replace('-','').ToLowerInvariant();$seen+=@{characters=$content.Length;sha256=$hash}
       if($hash -ceq $expected.displaySha256Utf8){$matched=$true;$sourcePattern=$pattern;break}
      }
    }
   }
   $clickReceipt.after=Source-Click-State $label $tabs $expectedNames $clickX $clickY;Save
   try{Check "Section $section tab $tab exact source text" $matched @{label=$name;expectedSha256=$expected.displaySha256Utf8;observed=$seen}}
   catch{
    $originalFailure=$_
    # Check has already persisted the original failed assertion and text hashes.
    # Snapshot that state before the diagnostic changes the viewport or selection.
    $clickReceipt.firstFailureScreenshot="source-first-failure-section-$section-tab-$tab.png"
    try{Shot "source-first-failure-section-$section-tab-$tab"}catch{$clickReceipt.firstFailureScreenshotError=$_.Exception.Message}
    try{$clickReceipt.interiorDiagnostic=Source-Interior-Diagnostic $tabs $codeScroll $name $expectedNames $expected}catch{$clickReceipt.interiorDiagnosticError=$_.Exception.Message}
    Save;throw $originalFailure
   }
   $visited[$name]=$true
   # Exact text alone cannot catch short documents centered in a tall viewport.
   # Read the first visible glyph without scrolling or changing the selection.
   $codeBounds=$codeScroll.Current.BoundingRectangle
   $codePattern=$codeScroll.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
   $outerBounds=Visible-Scroll-Bounds $outer
   $topGeometry=@{viewport=$codeBounds.ToString();verticalPercent=$codePattern.Current.VerticalScrollPercent;horizontalPercent=$codePattern.Current.HorizontalScrollPercent}
   $firstGlyph=[regex]::Match($content,'\S')
   if($firstGlyph.Success -and $codePattern.Current.VerticalScrollPercent -le .05 -and $codePattern.Current.HorizontalScrollPercent -le .05 -and $codeBounds.Top -ge $outerBounds.Top+80*$scale -and $codeBounds.Top+42*$scale -lt $outerBounds.Bottom){
    $range=$sourcePattern.DocumentRange.Clone()
    [void]$range.MoveEndpointByUnit([System.Windows.Automation.Text.TextPatternRangeEndpoint]::Start,[System.Windows.Automation.Text.TextUnit]::Character,$firstGlyph.Index)
    $range.MoveEndpointByRange([System.Windows.Automation.Text.TextPatternRangeEndpoint]::End,$range,[System.Windows.Automation.Text.TextPatternRangeEndpoint]::Start)
    [void]$range.MoveEndpointByUnit([System.Windows.Automation.Text.TextPatternRangeEndpoint]::End,[System.Windows.Automation.Text.TextUnit]::Character,1)
    $rectangles=$range.GetBoundingRectangles();$topGeometry.firstGlyphRectangles=$rectangles
    $leadingLines=($content.Substring(0,$firstGlyph.Index) -split "`n").Count-1
    $topGeometry.leadingLines=$leadingLines
    $topGeometry.expectedMaximumInset=(42+20*$leadingLines)*$scale
    $topGeometry.inset=if($rectangles.Count -gt 0 -and !$rectangles[0].IsEmpty){$rectangles[0].Top-$codeBounds.Top}else{$null}
    $clickReceipt.topGeometry=$topGeometry;Save
    Check 'Source first line retains authored top spacing' ($null -ne $topGeometry.inset -and $topGeometry.inset -ge 8*$scale -and $topGeometry.inset -le $topGeometry.expectedMaximumInset) @{label=$name;geometry=$topGeometry}
   }else{$topGeometry.notObserved='First line clipped or source viewport scrolled';$clickReceipt.topGeometry=$topGeometry;Save}
   $report.websiteSourceTabs+=@{section=$section;tab=$tab;label=$name;sha256=$expected.displaySha256Utf8;utc=[DateTime]::UtcNow.ToString('o')};[void](Website-Sample 'source-tab');Save
  }
  Mark "source-section-$section"
  $closers=@(Visible-Named '×' ([System.Windows.Automation.ControlType]::Button));Check 'Source close control visible uniquely' ($closers.Count -eq 1) $closers.Count;Click-Visible $closers[0];Start-Sleep -Seconds 1
  $report.websiteSourceSections+=@{section=$section;id=$drawer.id;tabs=$visited.Count;utc=[DateTime]::UtcNow.ToString('o')};Save
 }
 Check 'All authored source tabs qualified' ($report.websiteSourceTabs.Count -eq $sourceRows.Count) @{expected=$sourceRows.Count;actual=$report.websiteSourceTabs.Count}
 Check 'All six authored source sections were visited in order' (($report.websiteSourceSections.id -join ',') -ceq ($drawerPlan.id -join ',')) @($report.websiteSourceSections)
 $expectedVisits=@(for($i=0;$i -lt $drawerPlan.Count;$i++){foreach($row in $drawerPlan[$i].sources){[string]$i+'|'+$row.label+'|'+$row.displaySha256Utf8}})|Sort-Object
 $actualVisits=@($report.websiteSourceTabs|ForEach-Object{[string]$_.section+'|'+$_.label+'|'+$_.sha256})|Sort-Object
 Check 'Every authored source occurrence qualified in its own section' (($actualVisits -join "`n") -ceq ($expectedVisits -join "`n")) @{expected=$expectedVisits;actual=$actualVisits}
 }
 $route=Named 'OPEN CALCULATOR FULL SCREEN  ↗'|Select-Object -First 1;if($null -eq $route){$route=Named 'TRY THE CALCULATOR  ↗'|Select-Object -First 1};Check 'Authored local route link exists' ($null -ne $route) '/calculator/';Reveal $route $outer $false 0 $true
 $oldScrollIds=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)|ForEach-Object{Scroll-Id $_})
 $routeBounds=Visible-Scroll-Bounds $route;$routeViewport=Visible-Scroll-Bounds $outer
 $routePoint=Get-VisibleViewportPoint $routeBounds $routeViewport
 $routeContained=$null -ne $routePoint -and $routePoint.x -ge $routeBounds.Left -and $routePoint.x -lt $routeBounds.Right -and $routePoint.y -ge $routeBounds.Top -and $routePoint.y -lt $routeBounds.Bottom -and $routePoint.x -ge $routeViewport.Left -and $routePoint.x -lt $routeViewport.Right -and $routePoint.y -ge $routeViewport.Top -and $routePoint.y -lt $routeViewport.Bottom
 Check 'Route click is inside visible label and viewport' $routeContained @{point=$routePoint;label=$routeBounds.ToString();viewport=$routeViewport.ToString();outerY=$outerPattern.Current.VerticalScrollPercent}
 $routeWatch=[Diagnostics.Stopwatch]::StartNew();$retired=$false
 Point $routePoint.x $routePoint.y $true;Start-Sleep -Milliseconds 500
 while($routeWatch.Elapsed.TotalSeconds -lt 10){
  $currentIds=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)|ForEach-Object{Scroll-Id $_})
  $remaining=@($currentIds|Where-Object{$_ -in $oldScrollIds})
  if((@(Visible-Named 'CALCULATOR / LIVE')).Count -gt 0 -and !$remaining.Count){$retired=$true;break}
  Start-Sleep -Milliseconds 250
 }
 # Visited gallery content has authored exit timelines. Require actual eventual
 # retirement rather than incorrectly insisting it vanish on the first calculator tick.
 Check 'Local route retires old native scrollers after authored exit' $retired @{elapsedSeconds=$routeWatch.Elapsed.TotalSeconds;remaining=$remaining}
 Mark 'route-calculator';Click-Text '← BACK TO PAX';Start-Sleep -Seconds 2
 $newScrolls=@($root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
 $outer=@($newScrolls|Where-Object{Visible-Item $_}|Sort-Object @{Expression={$b=Visible-Scroll-Bounds $_;$b.Width*$b.Height};Descending=$true})|Select-Object -First 1
 Check 'Return home mounts a new native outer scroller' ($null -ne $outer -and (Scroll-Id $outer) -notin $oldScrollIds) $(if($outer){Scroll-Id $outer}else{'absent'})
 $outerPattern=$outer.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern);$beforeHome=$outerPattern.Current.VerticalScrollPercent
 # HomePage binds its offset to the surviving Example.page_scroll_y property.
 # A remount can therefore begin near the bottom; exercise available scroll
 # range rather than assuming the authored state reset to the top.
 Check 'Returned home has a native vertical scroll range' ($outerPattern.Current.VerticallyScrollable -and $beforeHome -ge 0 -and $beforeHome -le 100) $beforeHome
 $homeDirection=if($beforeHome -ge 50){1}else{-1};Wheel-At $outer $homeDirection $false 3
 $afterHome=$outerPattern.Current.VerticalScrollPercent
 Check 'Returned home responds to actual native wheel' $(if($homeDirection -gt 0){$afterHome -lt $beforeHome}else{$afterHome -gt $beforeHome}) @{before=$beforeHome;after=$afterHome;direction=$homeDirection}
 Mark 'route-returned-home'
 $report.websiteInteractionAcceptance=($report.websiteInteractionIssues.Count -eq 0);Mark 'website-interactions-complete';if($report.websiteInteractionIssues.Count){throw ($report.websiteInteractionIssues -join '; ')}
}
