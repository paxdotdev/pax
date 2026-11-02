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
New-Item -ItemType Directory -Force $out|Out-Null
$scratch=Join-Path $out ('source-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch|Out-Null
# Reuse the qualified SDK's pinned packages, but never overwrite its project,
# generated headers, intermediates or production outputs. No restore/download.
Copy-Item (Join-Path $sdk '*.h') $scratch
Copy-Item (Join-Path $PSScriptRoot 'rich-text-contract.cpp') $scratch
Copy-Item (Join-Path $PSScriptRoot 'app.manifest') $scratch
[xml]$project=Get-Content (Join-Path $sdk 'PaxHost.vcxproj') -Raw
$ns=New-Object Xml.XmlNamespaceManager($project.NameTable)
$ns.AddNamespace('m','http://schemas.microsoft.com/developer/msbuild/2003')
$project.SelectSingleNode('//m:HostPackagesRoot',$ns).InnerText=(Join-Path $sdk 'native-packages')+'\'
$project.SelectSingleNode('//m:ConfigurationType',$ns).InnerText='Application'
$project.SelectSingleNode('//m:TargetName',$ns).InnerText='RichTextContract'
$definitions=$project.SelectSingleNode('//m:ItemDefinitionGroup',$ns)
$manifest=$project.CreateElement('Manifest',$project.DocumentElement.NamespaceURI)
$manifestFiles=$project.CreateElement('AdditionalManifestFiles',$project.DocumentElement.NamespaceURI)
$manifestFiles.InnerText='app.manifest;%(AdditionalManifestFiles)'
[void]$manifest.AppendChild($manifestFiles);[void]$definitions.AppendChild($manifest)
foreach($node in @($project.SelectNodes('//m:ItemGroup/m:ClCompile',$ns))){[void]$node.ParentNode.RemoveChild($node)}
$group=$project.CreateElement('ItemGroup',$project.DocumentElement.NamespaceURI)
$compile=$project.CreateElement('ClCompile',$project.DocumentElement.NamespaceURI)
$compile.SetAttribute('Include','rich-text-contract.cpp');[void]$group.AppendChild($compile)
[void]$project.DocumentElement.AppendChild($group)
$path=Join-Path $scratch 'RichTextContract.vcxproj';$project.Save($path)
& $MSBuild $path /m:2 /v:minimal "/p:Platform=$Architecture" "/p:Configuration=$Configuration" "/p:OutputDirectory=$out"
if($LASTEXITCODE -ne 0){throw "Rich text contract build failed ($LASTEXITCODE)"}
Write-Output (Join-Path $out 'RichTextContract.exe')
