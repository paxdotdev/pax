$ErrorActionPreference = 'Stop'
# EC2Launch generates the Administrator password; none is in user data or state.
Enable-PSRemoting -Force -SkipNetworkProfileCheck
Get-ChildItem WSMan:\Localhost\Listener | Remove-Item -Recurse -Force
$cert = New-SelfSignedCertificate -DnsName 'pax-packer' -CertStoreLocation Cert:\LocalMachine\My
New-Item WSMan:\Localhost\Listener -Transport HTTPS -Address '*' -CertificateThumbPrint $cert.Thumbprint -Force | Out-Null
Set-Item WSMan:\Localhost\Service\AllowUnencrypted $false
Set-Item WSMan:\Localhost\Service\Auth\Basic $false
Set-Item WSMan:\Localhost\Service\Auth\Negotiate $true
New-NetFirewallRule -DisplayName 'Pax Packer WinRM TLS' -Direction Inbound -Protocol TCP -LocalPort 5986 -Action Allow | Out-Null
Set-Service WinRM -StartupType Automatic
Restart-Service WinRM
