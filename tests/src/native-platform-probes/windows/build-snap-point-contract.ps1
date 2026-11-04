param(
    [Parameter(Mandatory=$true)][string]$SdkSourceDirectory,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [ValidateSet('ARM64','x64')][string]$Architecture='x64',
    [ValidateSet('Debug','Release')][string]$Configuration='Debug',
    [string]$MSBuild='C:\BuildTools\MSBuild\Current\Bin\MSBuild.exe'
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$sdk=(Resolve-Path $SdkSourceDirectory).Path
$out=[IO.Path]::GetFullPath($OutputDirectory)
if(!(Test-Path (Join-Path $sdk 'snap_canvas.h'))){throw 'SDK source must contain the snap point Canvas header'}
New-Item -ItemType Directory -Force $out|Out-Null
$scratch=Join-Path $out ('source-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch|Out-Null
# Reuse the qualified SDK's pinned packages without restoring or modifying it.
Copy-Item (Join-Path $sdk '*.h') $scratch
Copy-Item (Join-Path $PSScriptRoot 'snap-point-contract.cpp') $scratch
Copy-Item (Join-Path $PSScriptRoot 'app.manifest') $scratch
[xml]$project=Get-Content (Join-Path $sdk 'PaxHost.vcxproj') -Raw
$ns=New-Object Xml.XmlNamespaceManager($project.NameTable)
$ns.AddNamespace('m','http://schemas.microsoft.com/developer/msbuild/2003')
$packages=Join-Path $sdk 'native-packages'
$receipt=Join-Path $sdk 'host-packages.json'
if(Test-Path $receipt){$packages=(Get-Content $receipt -Raw|ConvertFrom-Json).packagesDirectory}
if(-not (Test-Path $packages -PathType Container)){throw 'Build the host SDK dependencies before this contract.'}
$project.SelectSingleNode('//m:HostPackagesRoot',$ns).InnerText=$packages.TrimEnd('\','/')+'\'
$project.SelectSingleNode('//m:ConfigurationType',$ns).InnerText='Application'
$project.SelectSingleNode('//m:TargetName',$ns).InnerText='SnapPointContract'
$definitions=$project.SelectSingleNode('//m:ItemDefinitionGroup',$ns)
$manifest=$project.CreateElement('Manifest',$project.DocumentElement.NamespaceURI)
$manifestFiles=$project.CreateElement('AdditionalManifestFiles',$project.DocumentElement.NamespaceURI)
$manifestFiles.InnerText='app.manifest;%(AdditionalManifestFiles)'
[void]$manifest.AppendChild($manifestFiles);[void]$definitions.AppendChild($manifest)
foreach($node in @($project.SelectNodes('//m:ItemGroup/m:ClCompile',$ns))){[void]$node.ParentNode.RemoveChild($node)}
$group=$project.CreateElement('ItemGroup',$project.DocumentElement.NamespaceURI)
$compile=$project.CreateElement('ClCompile',$project.DocumentElement.NamespaceURI)
$compile.SetAttribute('Include','snap-point-contract.cpp');[void]$group.AppendChild($compile)
[void]$project.DocumentElement.AppendChild($group)
$path=Join-Path $scratch 'SnapPointContract.vcxproj';$project.Save($path)
& $MSBuild $path /m:2 /v:minimal "/p:Platform=$Architecture" "/p:Configuration=$Configuration" "/p:OutputDirectory=$out"
if($LASTEXITCODE -ne 0){throw "Snap point contract build failed ($LASTEXITCODE)"}
Write-Output (Join-Path $out 'SnapPointContract.exe')
