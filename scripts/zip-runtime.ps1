param([Parameter(Mandatory=$true)][string]$SourceRoot, [Parameter(Mandatory=$true)][string]$ArchivePath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$runtimeSource = (Resolve-Path -LiteralPath $SourceRoot).Path
$runtimeArchive = [IO.Path]::GetFullPath($ArchivePath)
if (Test-Path -LiteralPath $runtimeArchive) { throw 'Archive already exists; choose a fresh output candidate' }
[IO.Compression.ZipFile]::CreateFromDirectory($runtimeSource, $runtimeArchive, [IO.Compression.CompressionLevel]::Optimal, $false)
