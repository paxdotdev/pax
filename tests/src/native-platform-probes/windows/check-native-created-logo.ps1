param([Parameter(Mandatory=$true)][string]$Executable,[Parameter(Mandatory=$true)][string]$OutputDirectory,[string]$ExpectedHostSha256='',[string]$ExpectedExecutableSha256='',[string]$ExpectedCartridgeSha256='',[string]$WorkingDirectory='')
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
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
$oldLog=$env:PAX_WINDOWS_HOST_LOG;$env:PAX_WINDOWS_HOST_LOG=Join-Path $run 'native-host.log';$process=$null
$oldDpi=[PaxControlsInput]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$report=[ordered]@{result='running';cleanRetirement=$false;startedUtc=[DateTime]::UtcNow.ToString('o');exe=$Executable;sha256=(Get-FileHash $Executable).Hash;checks=@();observations=@();screenshots=@();timingScope='host callbacks/submission/publication, not GPU/display/DCV frames';samples=@();scrolls=@()}
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

function Pixel([double]$x,[double]$y){
 $origin=New-Object PaxControlsInput+POINT;[void][PaxControlsInput]::ClientToScreen($process.MainWindowHandle,[ref]$origin)
 $bmp=New-Object Drawing.Bitmap(1,1);$g=[Drawing.Graphics]::FromImage($bmp)
 try{$g.CopyFromScreen([int]($origin.X+$x*$scale),[int]($origin.Y+$y*$scale),0,0,$bmp.Size);$c=$bmp.GetPixel(0,0);return @($c.R,$c.G,$c.B)}finally{$g.Dispose();$bmp.Dispose()}
}
try {
 Check 'Interactive session' ([Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) ([Diagnostics.Process]::GetCurrentProcess().SessionId)
 $hostPath=Join-Path (Split-Path $Executable) 'PaxHost.dll';$hostHash=(Get-FileHash $hostPath).Hash;$report.hostSha256=$hostHash
 if($ExpectedHostSha256){Check 'Expected host file hash' ($hostHash -ieq $ExpectedHostSha256) $hostHash}
 if($ExpectedExecutableSha256){Check 'Expected executable file hash' ($report.sha256 -ieq $ExpectedExecutableSha256) $report.sha256}
 $process=Start-Process $Executable -WorkingDirectory $(if($WorkingDirectory){$WorkingDirectory}else{Split-Path $Executable}) -PassThru -RedirectStandardOutput (Join-Path $run 'stdout.log') -RedirectStandardError (Join-Path $run 'stderr.log');$handle=$process.Handle;$report.pid=$process.Id
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

 $report.runtimeModules=@($process.Modules|Where-Object{$_.ModuleName -match '^(PaxHost|Microsoft\.UI\.Xaml|Microsoft\.WindowsAppRuntime|vcruntime140|msvcp140).*\.dll$'}|ForEach-Object{@{name=$_.ModuleName;path=$_.FileName;fileVersion=$_.FileVersionInfo.FileVersion;sha256=(Get-FileHash $_.FileName).Hash}})
 $class=[Text.StringBuilder]::new(256);[void][PaxControlsInput]::GetClassNameW($process.MainWindowHandle,$class,$class.Capacity);Check 'Actual WinUI window selected' ($class.ToString() -eq 'WinUIDesktopWin32WindowClass') $class.ToString()
 $scale=[PaxControlsInput]::GetDpiForWindow($process.MainWindowHandle)/96.0;$report.dpi=96*$scale
 $root=[System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle);Focus;Start-Sleep -Seconds 2
 $sliders=@(Items ([System.Windows.Automation.ControlType]::Slider));Check 'Scaffold has genuine native playhead slider' ($sliders.Count -eq 1) $sliders.Count
 $slider=$sliders[0];$range=$slider.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern)
 Check 'Initial logo animation completes' ([Math]::Abs($range.Current.Value-1) -lt 0.0001) $range.Current.Value
 $replay=@(Items ([System.Windows.Automation.ControlType]::Text)|Where-Object{$_.Current.Name -ceq 'Replay'});Check 'Scaffold Replay text is visible' ($replay.Count -eq 1) $replay.Count
 $font=$replay[0].GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern).DocumentRange.GetAttributeValue([System.Windows.Automation.TextPattern]::FontNameAttribute)
 Check 'Scaffold reports bundled default Roboto style' (($font -split '#')[-1] -ceq 'Roboto') $font
 $light=Pixel 5 5;Check 'Initial light background' (@($light|Where-Object{[Math]::Abs($_-255) -gt 2}).Count -eq 0) $light;Shot 'initial'
 $slider.SetFocus();Tap 0x25
 Check 'Native slider arrow scrubs one authored step' ([Math]::Abs($range.Current.Value-0.999) -lt 0.0001) $range.Current.Value
 $bounds=$slider.Current.BoundingRectangle;$x=[int]($bounds.X+$bounds.Width-12*$scale);$y=[int]($bounds.Y+$bounds.Height/2)
 Point $x $y;Check 'Playhead pointer press delivered' ([PaxControlsInput]::Mouse(2) -eq 1) ''
 try{for($i=1;$i -le 8;$i++){Point ([int]($x+($bounds.X+$bounds.Width*0.25-$x)*$i/8)) $y;Start-Sleep -Milliseconds 30}}finally{[void][PaxControlsInput]::Mouse(4)}
 Settle;$scrub=$range.Current.Value;Check 'Native playhead drag changes progress' ($scrub -ge 0.15 -and $scrub -le 0.35) $scrub
 Start-Sleep -Milliseconds 450;Check 'Scrubbing cancels the running transition' ([Math]::Abs($range.Current.Value-$scrub) -lt 0.0001) $range.Current.Value
 Click-Item $replay[0];$restarted=$range.Current.Value;Check 'Replay starts a new animation' ($restarted -lt 0.95) $restarted
 Start-Sleep -Seconds 2;Check 'Replay animation reaches completion' ([Math]::Abs($range.Current.Value-1) -lt 0.0001) $range.Current.Value
 Button 'Use dark background';$dark=Pixel 5 5;Check 'Native background button updates rendered canvas' (@($dark|Where-Object{[Math]::Abs($_-17) -gt 2}).Count -eq 0) $dark
 Check 'Background binding updates native button label' (@(Items ([System.Windows.Automation.ControlType]::Button)|Where-Object{$_.Current.Name -ceq 'Use light background'}).Count -eq 1) 'Use light background';Shot 'dark'
 Button 'Use light background';$restored=Pixel 5 5;Check 'Background restores correctly' (@($restored|Where-Object{[Math]::Abs($_-255) -gt 2}).Count -eq 0) $restored
 $report.result='passed'
}catch{$report.result='failed';$report.error=$_.ToString();throw}finally{
 if($process -and -not $process.HasExited){[void]$process.CloseMainWindow();$closed=$process.WaitForExit(15000);$report.close=@{graceful=$closed;exitCode=if($closed){$process.ExitCode}else{$null}};if(!$closed){$process.Kill();$report.result='failed'}elseif($process.ExitCode -ne 0){$report.result='failed'}}
 if(Test-Path $env:PAX_WINDOWS_HOST_LOG){$tail=[IO.File]::ReadAllText($env:PAX_WINDOWS_HOST_LOG);$report.cleanRetirement=$tail.Contains('closed exports_drained=true') -and $tail.Contains('"allocatedBytes":0')}
 if(!$report.cleanRetirement){$report.result='failed'};$report.finishedUtc=[DateTime]::UtcNow.ToString('o');Save;$env:PAX_WINDOWS_HOST_LOG=$oldLog;[void][PaxControlsInput]::SetThreadDpiAwarenessContext($oldDpi)
}
if($report.result -ne 'passed'){throw $report.error}
