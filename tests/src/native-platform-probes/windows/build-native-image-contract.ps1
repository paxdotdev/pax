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
Copy-Item (Join-Path $PSScriptRoot 'native-image-contract.cpp') $scratch
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
$project.SelectSingleNode('//m:TargetName',$ns).InnerText='NativeImageContract'
$definitions=$project.SelectSingleNode('//m:ItemDefinitionGroup',$ns)
$manifest=$project.CreateElement('Manifest',$project.DocumentElement.NamespaceURI)
$manifestFiles=$project.CreateElement('AdditionalManifestFiles',$project.DocumentElement.NamespaceURI)
$manifestFiles.InnerText='app.manifest;%(AdditionalManifestFiles)'
[void]$manifest.AppendChild($manifestFiles);[void]$definitions.AppendChild($manifest)
foreach($node in @($project.SelectNodes('//m:ItemGroup/m:ClCompile',$ns))){[void]$node.ParentNode.RemoveChild($node)}
$group=$project.CreateElement('ItemGroup',$project.DocumentElement.NamespaceURI)
$compile=$project.CreateElement('ClCompile',$project.DocumentElement.NamespaceURI)
$compile.SetAttribute('Include','native-image-contract.cpp');[void]$group.AppendChild($compile)
[void]$project.DocumentElement.AppendChild($group)
$path=Join-Path $scratch 'NativeImageContract.vcxproj';$project.Save($path)
& $MSBuild $path /m:2 /v:minimal "/p:Platform=$Architecture" "/p:Configuration=$Configuration" "/p:OutputDirectory=$out"
if($LASTEXITCODE -ne 0){throw "Native image contract build failed ($LASTEXITCODE)"}
Write-Output (Join-Path $out 'NativeImageContract.exe')

$assets=Join-Path $out 'assets'
New-Item -ItemType Directory -Force $assets|Out-Null
# Deterministic 64x32 RGBA fixture: opaque red left half, blue right half.
[IO.File]::WriteAllBytes((Join-Path $assets 'halves.png'),[Convert]::FromBase64String('iVBORw0KGgoAAAANSUhEUgAAAEAAAAAgCAYAAACinX6EAAAARElEQVR4nO3QsQ0AAAzCMP5/ml5RsXjIDHKadNl4vvMHAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwH8HKHrw4mwybOwAAAAASUVORK5CYII='))
for($i=0;$i -lt 10;$i++){Copy-Item (Join-Path $assets 'halves.png') (Join-Path $assets "burst-$i.png")}
'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="16" viewBox="0 0 64 16" preserveAspectRatio="none"><defs><linearGradient id="fade"><stop offset="0" stop-color="#00ff00"/><stop offset="1" stop-color="#00ff00" stop-opacity="0"/></linearGradient></defs><rect width="64" height="16" fill="url(#fade)"/></svg>'|Set-Content (Join-Path $assets 'fade.svg') -Encoding UTF8
'not an image'|Set-Content (Join-Path $assets 'corrupt.png') -Encoding ASCII

# A larger intrinsic raster proves that layout does not clip before downscaling.
[IO.File]::WriteAllBytes((Join-Path $assets 'large-halves.png'),[Convert]::FromBase64String('iVBORw0KGgoAAAANSUhEUgAAAgAAAAEACAYAAADFkM5nAAAG20lEQVR4nO3OsQ3AQBAEoeu/6XcZs5IJyLl394B/GigAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABU+gFQGSgAlX4AVAYKQKUfAJWBAlDpB0BloABEPsoLO8VuWpdsAAAAAElFTkSuQmCC'))

'<svg xmlns="http://www.w3.org/2000/svg" width="10000" height="10000" viewBox="0 0 10000 10000"><rect width="5000" height="10000" fill="red"/><rect x="5000" width="5000" height="10000" fill="blue"/></svg>'|Set-Content (Join-Path $assets 'large-authored.svg') -Encoding UTF8
