param([Parameter(Mandatory=$true)][string]$Executable,[Parameter(Mandatory=$true)][string]$OutputDirectory,[string]$ExpectedHostSha256='',[string]$ExpectedExecutableSha256='',[string]$ExpectedCartridgeSha256='',[string]$FontMetricsExecutable='',[string]$CasesPath='',[switch]$SelectionDiagnostic)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
if([string]::IsNullOrEmpty($CasesPath)){$CasesPath=Join-Path $PSScriptRoot '..\navigation-cases.json'}
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Windows.Forms,System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PaxControlsInput {
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct CURSORINFO { public int Size,Flags; public IntPtr Cursor; public POINT Position; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int X,Y; public uint Data,Flags,Time; public IntPtr Extra; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort Key,Scan; public uint Flags,Time; public IntPtr Extra; }
    [StructLayout(LayoutKind.Explicit)] public struct UNION { [FieldOffset(0)] public MOUSEINPUT Mouse; [FieldOffset(0)] public KEYBDINPUT Keyboard; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint Type; public UNION Data; }
    [DllImport("user32.dll",SetLastError=true)] static extern uint SendInput(uint count, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr window,System.Text.StringBuilder value,int count);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr window, ref POINT point);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr window,int x,int y,int width,int height,bool repaint);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr window,uint flags);
    [DllImport("user32.dll")] public static extern bool GetCursorInfo(ref CURSORINFO info);
    [DllImport("user32.dll")] public static extern IntPtr LoadCursor(IntPtr instance,IntPtr id);
    public static uint Key(ushort key,bool up) {
        INPUT value=new INPUT(); value.Type=1; value.Data.Keyboard.Key=key; value.Data.Keyboard.Flags=up?2u:0u;
        return SendInput(1,new INPUT[]{value},Marshal.SizeOf(typeof(INPUT)));
    }
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int index);
    public static uint Move(int x,int y) {
        INPUT value=new INPUT(); value.Data.Mouse.Flags=0xC001;
        value.Data.Mouse.X=(int)Math.Round((x-GetSystemMetrics(76))*65535.0/(GetSystemMetrics(78)-1));
        value.Data.Mouse.Y=(int)Math.Round((y-GetSystemMetrics(77))*65535.0/(GetSystemMetrics(79)-1));
        return SendInput(1,new INPUT[]{value},Marshal.SizeOf(typeof(INPUT)));
    }
    public static uint Mouse(uint flags, int data=0) {
        INPUT value=new INPUT(); value.Data.Mouse.Flags=flags; value.Data.Mouse.Data=unchecked((uint)data);
        return SendInput(1,new INPUT[]{value},Marshal.SizeOf(typeof(INPUT)));
    }
    public static uint Click() {
        INPUT down=new INPUT(),up=new INPUT(); down.Data.Mouse.Flags=2; up.Data.Mouse.Flags=4;
        return SendInput(2,new INPUT[]{down,up},Marshal.SizeOf(typeof(INPUT)));
    }
}
'@
$Executable=(Resolve-Path $Executable).Path
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ([Guid]::NewGuid().ToString());New-Item -ItemType Directory -Force $run|Out-Null
$oldNavigationLog=$env:PAX_NAVIGATION_LOG;$env:PAX_NAVIGATION_LOG=Join-Path $run 'route-events.jsonl';$oldLog=$env:PAX_WINDOWS_HOST_LOG;$env:PAX_WINDOWS_HOST_LOG=Join-Path $run 'native-host.log';$process=$null
$oldDpi=[PaxControlsInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$report=[ordered]@{result='running';cleanRetirement=$false;startedUtc=[DateTime]::UtcNow.ToString('o');exe=$Executable;sha256=(Get-FileHash $Executable).Hash;checks=@();observations=@();screenshots=@();timingScope='host callbacks/submission/publication, not GPU/display/DCV frames';samples=@();scrolls=@();clientApartment=[Threading.Thread]::CurrentThread.GetApartmentState().ToString()}
function Save { $report|ConvertTo-Json -Depth 12|Set-Content (Join-Path $run 'result.json') -Encoding UTF8 }
function Check($name,[bool]$ok,$detail){$report.checks+=@{name=$name;passed=$ok;detail=$detail};Save;if(-not $ok){throw "$name failed: $($detail|ConvertTo-Json -Compress -Depth 5)"}}
function Items($type){$c=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$type);$a=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$c);for($i=0;$i -lt $a.Count;$i++){,$a.Item($i)}}
function Settle {Start-Sleep -Milliseconds 350}
function Focus {
 [void][PaxControlsInput]::SetForegroundWindow($process.MainWindowHandle);Start-Sleep -Milliseconds 100
 if([PaxControlsInput]::GetForegroundWindow() -ne $process.MainWindowHandle){$r=New-Object PaxControlsInput+RECT;if(-not [PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$r)){throw 'No window'};Point ($r.Left+100) ($r.Top+16) $true}
 Check 'Fixture owns foreground' ([PaxControlsInput]::GetForegroundWindow() -eq $process.MainWindowHandle) $process.Id
}
function Point([int]$x,[int]$y,[bool]$click=$false){
 $p=New-Object PaxControlsInput+POINT;$p.X=$x;$p.Y=$y;$owner=[PaxControlsInput]::GetAncestor([PaxControlsInput]::WindowFromPoint($p),2)
 if($owner -ne $process.MainWindowHandle){throw "Input target is occluded: $x,$y owner=$owner"}
 if([PaxControlsInput]::Move($x,$y) -ne 1){throw 'Absolute pointer movement injection failed'};Start-Sleep -Milliseconds 80;$actual=New-Object PaxControlsInput+POINT
 if(-not [PaxControlsInput]::GetCursorPos([ref]$actual) -or $actual.X -ne $x -or $actual.Y -ne $y){throw 'Pointer did not reach target'}
 if($click -and [PaxControlsInput]::Click() -ne 2){throw 'Click injection failed'};Start-Sleep -Milliseconds 100
}
function Local([double]$x,[double]$y,[bool]$click=$false){$p=New-Object PaxControlsInput+POINT;if(-not [PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$p)){throw 'No client origin'};Point ([int][Math]::Round($p.X+$x*$scale)) ([int][Math]::Round($p.Y+$y*$scale)) $click}
function Tap([int]$key){if([PaxControlsInput]::GetForegroundWindow() -ne $process.MainWindowHandle){throw 'Lost foreground'};if([PaxControlsInput]::Key($key,$false) -ne 1 -or [PaxControlsInput]::Key($key,$true) -ne 1){throw 'Key injection failed'};Settle}
function Click-Item($item){$b=$item.Current.BoundingRectangle;Point ([int]($b.X+$b.Width/2)) ([int]($b.Y+$b.Height/2)) $true;Settle}
function Button([string]$name){$a=@(Items ([System.Windows.Automation.ControlType]::Button)|Where-Object{$_.Current.Name -ceq $name});if($a.Count -ne 1){throw "Button $name ambiguous"};Click-Item $a[0]}
function Shot($name){Focus;$b=New-Object PaxControlsInput+RECT;[void][PaxControlsInput]::GetWindowRect($process.MainWindowHandle,[ref]$b);$bmp=New-Object Drawing.Bitmap(($b.Right-$b.Left),($b.Bottom-$b.Top));$g=[Drawing.Graphics]::FromImage($bmp);try{$g.CopyFromScreen($b.Left,$b.Top,0,0,$bmp.Size);$bmp.Save((Join-Path $run "$name.png"),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$bmp.Dispose()};$report.screenshots+="$name.png";Save}

function Attr($element,$name){$p=$element.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern);$id=[System.Windows.Automation.TextPattern]::$name;return $p.DocumentRange.GetAttributeValue($id)}
function Canonical($value) {
 if($null -eq $value){return 'null'}
 if($value -is [string]){return (ConvertTo-Json -InputObject $value -Compress)}
 if($value -is [System.Management.Automation.PSCustomObject]){$pairs=@();foreach($p in @($value.PSObject.Properties|Sort-Object Name)){$pairs+=((ConvertTo-Json -InputObject $p.Name -Compress)+':'+(Canonical $p.Value))};return '{'+($pairs -join ',')+'}'}
 if($value -is [System.Collections.IEnumerable]){$items=@();foreach($v in $value){$items+=,(Canonical $v)};return '['+($items -join ',')+']'}
 return (ConvertTo-Json -InputObject $value -Compress)
}
function Route($expected) {
 $deadline=[DateTime]::UtcNow.AddSeconds(5);$actual=$null;$name=''
 do {
  $labels=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -like '{*path_segments*'})
  if($labels.Count -eq 1){$name=$labels[0].Current.Name;$actual=$name|ConvertFrom-Json}
  if($actual -and (Canonical $actual) -ceq (Canonical $expected)){break}
  Start-Sleep -Milliseconds 100
 } while([DateTime]::UtcNow -lt $deadline)
 Check 'Visible native route matches expected location' ($actual -and (Canonical $actual) -ceq (Canonical $expected)) @{actual=$actual;expected=$expected}
 $lines=@(Get-Content $env:PAX_NAVIGATION_LOG -Encoding UTF8 -ErrorAction Stop);$logged=$lines[-1]|ConvertFrom-Json
 Check 'Rust Router log matches visible native route' ((Canonical $logged) -ceq (Canonical $expected)) $logged
 $report.observations+=@{route=$actual;logLines=$lines.Count};Save
}
try {
 Check 'Interactive session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
 # Windows PowerShell 5 cannot materialize JSON objects with an empty key.
 # The shared file includes such native parser edge cases; this UI gate uses
 # only initial/fixture_steps, while the C++ contract validates the full file.
 Add-Type -AssemblyName System.Web.Extensions
 $json=[Web.Script.Serialization.JavaScriptSerializer]::new();$allCases=$json.DeserializeObject([IO.File]::ReadAllText((Resolve-Path $CasesPath),[Text.Encoding]::UTF8))
 $cases=($json.Serialize(@{initial=$allCases['initial'];fixture_steps=$allCases['fixture_steps']}))|ConvertFrom-Json
 $hostPath=Join-Path (Split-Path $Executable) 'PaxHost.dll';$hostHash=(Get-FileHash $hostPath).Hash;$report.hostSha256=$hostHash
 if($ExpectedHostSha256){Check 'Expected host file hash' ($hostHash -ieq $ExpectedHostSha256) $hostHash}
 if($ExpectedExecutableSha256){Check 'Expected executable file hash' ($report.sha256 -ieq $ExpectedExecutableSha256) $report.sha256}
 $process=Start-Process $Executable -WorkingDirectory (Split-Path $Executable) -PassThru -RedirectStandardOutput (Join-Path $run 'stdout.log') -RedirectStandardError (Join-Path $run 'stderr.log');$handle=$process.Handle;$report.pid=$process.Id
 $until=[DateTime]::UtcNow.AddSeconds(60);do{Start-Sleep -Milliseconds 200;$process.Refresh();if($process.HasExited){throw 'App exited before first frame'}}while(($process.MainWindowHandle -eq [IntPtr]::Zero -or -not(Test-Path $env:PAX_WINDOWS_HOST_LOG) -or -not([IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published'))) -and [DateTime]::UtcNow -lt $until)
 Check 'Actual Pax runtime first frame' ([IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG).Contains('first_frame_published')) $process.MainWindowHandle.ToInt64()
 $report.loadedExecutablePath=$process.MainModule.FileName;$report.loadedExecutableSha256=(Get-FileHash $report.loadedExecutablePath).Hash
 Check 'Expected executable actually running' ($report.loadedExecutablePath -ieq $Executable -and $report.loadedExecutableSha256 -ieq $report.sha256) @{path=$report.loadedExecutablePath;sha256=$report.loadedExecutableSha256}
 $cartridgeName=[IO.Path]::GetFileNameWithoutExtension($Executable).Replace('-','_')+'.dll'
 $cartridges=@($process.Modules|Where-Object{$_.ModuleName -ieq $cartridgeName})
 $report.loadedCartridgePath=if($cartridges.Count -eq 1){$cartridges[0].FileName}else{''}
 $report.cartridgeSha256=if($cartridges.Count -eq 1){(Get-FileHash $cartridges[0].FileName).Hash}else{''}
 if($ExpectedCartridgeSha256){Check 'Expected cartridge actually loaded' ($cartridges.Count -eq 1 -and $report.cartridgeSha256 -ieq $ExpectedCartridgeSha256) @{path=$report.loadedCartridgePath;sha256=$report.cartridgeSha256;expected=$ExpectedCartridgeSha256}}
 $loaded=@($process.Modules|Where-Object{$_.ModuleName -ieq 'PaxHost.dll'});Check 'Expected native DLL loaded' ($loaded.Count -eq 1 -and (Get-FileHash $loaded[0].FileName).Hash -ieq $hostHash) $hostHash
 $class=[Text.StringBuilder]::new(256);[void][PaxControlsInput]::GetClassNameW($process.MainWindowHandle,$class,$class.Capacity);Check 'Actual WinUI window selected' ($class.ToString() -eq 'WinUIDesktopWin32WindowClass') $class.ToString()
 $scale=[PaxControlsInput]::GetDpiForWindow($process.MainWindowHandle)/96.0;$report.dpi=96*$scale
 $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle);Focus;Start-Sleep -Seconds 2;Route $cases.initial;Shot 'initial'
 foreach($step in $cases.fixture_steps){
  if($step.label -ceq 'Home link'){
   $homeLinkItems=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -ceq 'Home link'});Check 'Native Link text is present' ($homeLinkItems.Count -eq 1) $homeLinkItems.Count
   $before=@(Get-Content $env:PAX_NAVIGATION_LOG -Encoding UTF8).Count
   $bounds=$homeLinkItems[0].Current.BoundingRectangle;$midY=[int]($bounds.Y+$bounds.Height/2)
   Point ([int]$bounds.X+2) $midY
   if([PaxControlsInput]::Mouse(2) -ne 1){throw 'Selection press failed'}
   try{for($n=1;$n -le 12;$n++){Point ([int]($bounds.X+2+($bounds.Width-4)*$n/12)) $midY;Start-Sleep -Milliseconds 25}}finally{if([PaxControlsInput]::Mouse(4) -ne 1){throw 'Selection release failed'}}
   Settle
   if($SelectionDiagnostic){
    $diagnostic=Join-Path $PSScriptRoot 'inspect-native-selection.ps1';$diagnosticOutput=Join-Path $run 'selection-mta.json'
    $d=Start-Process powershell.exe -NoNewWindow -PassThru -ArgumentList ('-NoProfile -Mta -File "'+$diagnostic+'" -WindowHandle '+$process.MainWindowHandle.ToInt64()+' -OutputFile "'+$diagnosticOutput+'"') -RedirectStandardOutput (Join-Path $run 'selection-mta.stdout') -RedirectStandardError (Join-Path $run 'selection-mta.stderr');$dh=$d.Handle
    $finished=$d.WaitForExit(15000);if(!$finished){$d.Kill();$d.WaitForExit()}
    $report.observations+=@{selectionClient=@{apartment='MTA';completed=$finished;exitCode=$d.ExitCode;output='selection-mta.json'}};Save
    $nativeSelection=Join-Path (Split-Path $FontMetricsExecutable) 'selection-uia.exe'
    if(Test-Path $nativeSelection){
     foreach($maximum in @('4096','-1')){
      $nativeOutput=Join-Path $run ('selection-native-'+$maximum+'.jsonl')
      $native=Start-Process $nativeSelection -NoNewWindow -PassThru -ArgumentList @([string]$process.Id,$homeLinkItems[0].Current.AutomationId,$maximum,('"'+$nativeOutput+'"'));$nh=$native.Handle
      $nativeFinished=$native.WaitForExit(20000);if(!$nativeFinished){$native.Kill();$native.WaitForExit()}
      $report.observations+=@{nativeSelectionClient=@{apartment='MTA';maximum=$maximum;completed=$nativeFinished;exitCode=$native.ExitCode;helperSha256=(Get-FileHash $nativeSelection).Hash;output=[IO.Path]::GetFileName($nativeOutput)}};Save
     }
    }

   }
   # Exercise the native selection's Copy command without exposing the user's
   # clipboard. UIA selected-range inspection is isolated above because a
   # provider/client crash must not bypass cleanup of this acceptance process.
   $priorClipboard=[Windows.Forms.Clipboard]::GetDataObject()
   try{
    [Windows.Forms.Clipboard]::Clear()
    if([PaxControlsInput]::Key(0x11,$false) -ne 1){throw 'Copy modifier failed'}
    try{Tap 0x43}finally{[void][PaxControlsInput]::Key(0x11,$true)}
    $selected=[Windows.Forms.Clipboard]::GetText()
    Check 'Native selectable Link copies exact dragged text' ($selected -ceq 'Home link') @{matchesExpected=($selected -ceq 'Home link');length=$selected.Length}
   }finally{if($null -ne $priorClipboard){[Windows.Forms.Clipboard]::SetDataObject($priorClipboard,$true)}else{[Windows.Forms.Clipboard]::Clear()}}
   Check 'Selection drag does not activate Link' (@(Get-Content $env:PAX_NAVIGATION_LOG -Encoding UTF8).Count -eq $before) $before
   Click-Item $homeLinkItems[0]
   Check 'Single native tap emits one route change' (@(Get-Content $env:PAX_NAVIGATION_LOG -Encoding UTF8).Count -eq ($before+1)) @{before=$before;after=@(Get-Content $env:PAX_NAVIGATION_LOG -Encoding UTF8).Count}
  }
  else {Button $step.label}
  Route $step.expected
 }
 foreach($definition in @(@{name='Default Roboto:*';weight=400;italic=$false},@{name='Default bold weight 700 @@@@@@@@';weight=700;italic=$false},@{name='Default italic face @@@@@@@@';weight=400;italic=$true})){
  $labels=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -like $definition.name});Check 'Default-font sample is visible' ($labels.Count -eq 1 -and $labels[0].Current.BoundingRectangle.Width -gt 10) $definition.name
  $family=Attr $labels[0] 'FontNameAttribute';$weight=Attr $labels[0] 'FontWeightAttribute';$italic=Attr $labels[0] 'IsItalicAttribute'
  Check 'Native style reports bundled Roboto resource' (($family -split '#')[-1] -ceq 'Roboto') $family
  Check 'Native default font weight' ([int]$weight -eq $definition.weight) $weight
  Check 'Native default italic face' ([bool]$italic -eq $definition.italic) $italic
  if($FontMetricsExecutable){
   $fontName=if($definition.italic){'Roboto-Italic.ttf'}else{'Roboto.ttf'}
   $exeRoot=Split-Path $Executable;$candidates=@();foreach($folder in @('assets','pax-fonts')){if(Test-Path (Join-Path $exeRoot $folder)){$candidates+=@(Get-ChildItem (Join-Path $exeRoot $folder) -Recurse -File -Filter $fontName)}}
   Check 'One exact executable-relative default font file' ($candidates.Count -eq 1) @($candidates|ForEach-Object{$_.FullName})
   $font=$candidates[0].FullName;$id=$labels[0].Current.AutomationId;$content=$labels[0].Current.Name
   $tag='font-'+$definition.weight+'-'+$definition.italic;$stdout=Join-Path $run ($tag+'.jsonl');$stderr=Join-Path $run ($tag+'.stderr')
   $fontArgs=@([string]$process.Id,$id,('"'+$font+'"'),'Roboto','26',[string]$definition.weight,$(if($definition.italic){'1'}else{'0'}),('"'+$content+'"'),'@@@@@@@@')
   $helper=Start-Process $FontMetricsExecutable -ArgumentList $fontArgs -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr;$helperHandle=$helper.Handle
   if(!$helper.WaitForExit(20000)){$helper.Kill();throw 'Native font metric helper deadline'}
   $complete=@(Get-Content $stdout -Encoding UTF8|ForEach-Object{$_|ConvertFrom-Json}|Where-Object{$_.PSObject.Properties.Name -contains 'stage' -and $_.stage -eq 'complete'})
   Check 'DirectWrite glyph comparison completed' ($helper.ExitCode -eq 0 -and $complete.Count -eq 1) @{exit=$helper.ExitCode;output=$stdout}
   $metric=$complete[0];$report.observations+=@{fontMetric=$metric;stagedSha256=(Get-FileHash $font).Hash;helperSha256=(Get-FileHash $FontMetricsExecutable).Hash}
   Check 'Native glyphs match exact bundled face and exclude fallbacks' ($metric.qualified -eq $true -and !$metric.systemFamilyPresent -and $metric.pid -eq $process.Id -and $metric.hwnd -eq $process.MainWindowHandle.ToInt64() -and $metric.automationId -ceq $id -and $metric.fontFile -ceq $font -and $metric.text -ceq $content -and $metric.family -ceq 'Roboto') $metric
  }
 }
 Shot 'final';$report.result='passed'
}catch{$report.result='failed';$report.error=$_.ToString();throw}finally{
 if($process -and -not $process.HasExited){[void]$process.CloseMainWindow();$closed=$process.WaitForExit(15000);$report.close=@{graceful=$closed;exitCode=if($closed){$process.ExitCode}else{$null}};if(!$closed){$process.Kill();$report.result='failed'}elseif($process.ExitCode -ne 0){$report.result='failed'}}
 if(Test-Path $env:PAX_WINDOWS_HOST_LOG){$tail=[IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG);$report.cleanRetirement=$tail.Contains('closed exports_drained=true') -and $tail.Contains('"allocatedBytes":0')}
 if(!$report.cleanRetirement){$report.result='failed'};$report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save;$env:PAX_WINDOWS_HOST_LOG=$oldLog;$env:PAX_NAVIGATION_LOG=$oldNavigationLog;[void][PaxControlsInput]::SetThreadDpiAwarenessContext($oldDpi)
}
if($report.result -ne 'passed'){throw $report.error}
