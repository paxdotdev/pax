$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = 'C:\PaxGpuLab'
$inputs = Get-Content "$root\inputs.json" -Raw | ConvertFrom-Json
$nvidia = Get-Command nvidia-smi.exe -ErrorAction SilentlyContinue
if (-not $nvidia) {
    $nvidiaPath = 'C:\Windows\System32\nvidia-smi.exe'
} else { $nvidiaPath = $nvidia.Source }
$gpu = & $nvidiaPath --query-gpu=name,driver_version,driver_model.current --format=csv,noheader
if ($LASTEXITCODE -ne 0 -or -not $gpu) { throw 'NVIDIA graphics driver required' }
foreach ($row in ($gpu | ConvertFrom-Csv -Header Name, Driver, Model)) {
    if ($row.Model.Trim() -ne 'WDDM') { throw 'NVIDIA WDDM driver required' }
    if ($row.Driver.Trim() -ne $inputs.artifacts.grid.version) { throw 'Driver version differs from the manifest' }
}
$gpu | Set-Content "$root\gpu.txt"
Start-Service dcvserver
Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber | ConvertTo-Json | Set-Content "$root\os.json"
Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' |
    Where-Object { $_.PSObject.Properties.Name -contains 'DisplayName' } |
    Select-Object DisplayName, DisplayVersion | ConvertTo-Json | Set-Content "$root\packages.json"

# Preserve the communicator until Sysprep completes. On the first image boot,
# remove only the temporary Packer listener/certificate and disable its service.
$cleanup = @'
$ErrorActionPreference = 'Stop'
Get-ChildItem WSMan:\Localhost\Listener | Where-Object { $_.Keys -contains 'Transport=HTTPS' } | Remove-Item -Recurse -Force
Get-ChildItem Cert:\LocalMachine\My | Where-Object Subject -eq 'CN=pax-packer' | Remove-Item -Force
Get-NetFirewallRule -DisplayName 'Pax Packer WinRM TLS' -ErrorAction SilentlyContinue | Remove-NetFirewallRule
Set-Service WinRM -StartupType Disabled
Stop-Service WinRM -Force
Unregister-ScheduledTask -TaskName PaxGpuLabRemoveBuilderAccess -Confirm:$false
Remove-Item -LiteralPath $PSCommandPath -Force
'@
$cleanup | Set-Content "$root\remove-builder-access.ps1"
$action = New-ScheduledTaskAction -Execute powershell.exe -Argument "-NoProfile -ExecutionPolicy Bypass -File $root\remove-builder-access.ps1"
Register-ScheduledTask -TaskName PaxGpuLabRemoveBuilderAccess -Action $action -Trigger (New-ScheduledTaskTrigger -AtStartup) -User SYSTEM -RunLevel Highest -Force | Out-Null
