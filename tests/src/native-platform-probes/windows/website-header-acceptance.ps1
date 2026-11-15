# Uses the parent probe's owned desktop/input and normal-close checks. Pixel
# expectations come from SiteHeader's authored boxes, independently of UIA text.
function Invoke-WebsiteHeaderAcceptance {
 $report.websiteHeaderSamples=@()
 function Header-Sample([string]$name,[bool]$compact,[bool]$assertSettled=$true) {
  Focus
  $origin=New-Object PaxControlsInput+POINT;$client=New-Object PaxControlsInput+RECT
  [void][PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$origin)
  [void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$client)
  $margin=if($compact){20}else{40};$boxWidth=if($compact){116}else{150}
  $boxTop=if($compact){14}else{20};$boxHeight=if($compact){36}else{40};$navHeight=if($compact){64}else{80}
  $left=$client.Right-($margin+$boxWidth)*$scale;$right=$client.Right-$margin*$scale
  $expected=@{left=$left;top=$boxTop*$scale;right=$right;bottom=($boxTop+$boxHeight)*$scale}
  $labels=@(Visible-Named 'Get Started')
  Check 'Header exposes one native CTA label' ($labels.Count -eq 1) $labels.Count
  $b=$labels[0].Current.BoundingRectangle
  $native=@{left=$b.Left-$origin.X;top=$b.Top-$origin.Y;right=$b.Right-$origin.X;bottom=$b.Bottom-$origin.Y}
  $bitmap=New-Object Drawing.Bitmap($client.Right,[int][Math]::Ceiling($navHeight*$scale))
  $graphics=[Drawing.Graphics]::FromImage($bitmap);$rows=@()
  try {
   $graphics.CopyFromScreen($origin.X,$origin.Y,0,0,$bitmap.Size)
   # Sample both padding columns, away from glyphs and antialiased edges.
   # A bright neutral fill is enough; exact color would encode driver lighting.
   for($y=0;$y -lt $bitmap.Height;$y++) {
    $bright=$true
    foreach($x in @([int][Math]::Round($left+6*$scale),[int][Math]::Round($right-6*$scale))) {
     $c=$bitmap.GetPixel($x,$y)
     if([Math]::Min($c.R,[Math]::Min($c.G,$c.B)) -lt 180 -or [Math]::Max($c.R,[Math]::Max($c.G,$c.B))-[Math]::Min($c.R,[Math]::Min($c.G,$c.B)) -gt 35){$bright=$false}
    }
    if($bright){$rows+=,$y}
   }
   $bitmap.Save((Join-Path $run ($name+'-header.png')),[Drawing.Imaging.ImageFormat]::Png)
  } finally {$graphics.Dispose();$bitmap.Dispose()}
  $paint=if($rows.Count){@{top=$rows[0];bottom=$rows[-1]+1;rows=$rows.Count}}else{$null}
  $detail=@{name=$name;utc=[DateTime]::UtcNow.ToString('o');compact=$compact;dpiScale=$scale;expected=$expected;native=$native;paint=$paint;outerY=$outerPattern.Current.VerticalScrollPercent}
  $report.websiteHeaderSamples+=,$detail;$report.screenshots+=($name+'-header.png');Save
  if(!$assertSettled){return}
  $tolerance=2*$scale
  Observe ($name+': vector background reaches authored box') ($null -ne $paint -and [Math]::Abs($paint.top-$expected.top) -le $tolerance -and [Math]::Abs($paint.bottom-$expected.bottom) -le $tolerance -and $paint.rows -ge $boxHeight*$scale-$tolerance) $detail
  Observe ($name+': native label remains centered') ([Math]::Abs(($native.left+$native.right-$expected.left-$expected.right)/2) -le 4*$scale -and [Math]::Abs(($native.top+$native.bottom-$expected.top-$expected.bottom)/2) -le 4*$scale) $detail
 }
 foreach($layout in @(@{name='desktop';width=1280;compact=$false},@{name='compact';width=700;compact=$true},@{name='desktop-return';width=1280;compact=$false})) {
  $wr=New-Object PaxControlsInput+RECT;$cr=New-Object PaxControlsInput+RECT
  [void][PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$wr)
  [void][PaxControlsInput]::GetClientRect($process.MainWindowHandle,[ref]$cr)
  [void][PaxControlsInput]::MoveWindow($process.MainWindowHandle,$wr.Left,$wr.Top,[int]($layout.width*$scale+$wr.Right-$wr.Left-$cr.Right),$wr.Bottom-$wr.Top,$true)
  Start-Sleep -Seconds 2
  # Re-enter through the real scroll/reveal transition on every layout.
  for($n=0;$n -lt 40 -and $outerPattern.Current.VerticalScrollPercent -gt .05;$n++){Wheel-At $outer 1 $false 20}
  Check 'Header replay reaches hero origin' ($outerPattern.Current.VerticalScrollPercent -le .05) $outerPattern.Current.VerticalScrollPercent
  Start-Sleep -Seconds 2;Mark ($layout.name+'-hero')
  for($n=0;$n -lt 20;$n++) {
   Wheel-At $outer -1 $false 5
   $labels=@(Visible-Named 'Get Started')
   if($labels.Count -eq 1 -and $labels[0].Current.BoundingRectangle.Top -gt $wr.Top+30){break}
  }
  Header-Sample ($layout.name+'-revealed') $layout.compact $false
  Start-Sleep -Seconds 2;Header-Sample ($layout.name+'-settled-2s') $layout.compact
  Start-Sleep -Seconds 8;Header-Sample ($layout.name+'-settled-10s') $layout.compact
  Mark ($layout.name+'-settled')
 }
 $report.websiteInteractionAcceptance=($report.websiteInteractionIssues.Count -eq 0);Save
 if($report.websiteInteractionIssues.Count){throw ($report.websiteInteractionIssues -join '; ')}
}
