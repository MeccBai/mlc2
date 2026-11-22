#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release')][string]$Profile = 'debug',
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ([string]::IsNullOrWhiteSpace($env:MLC)) { throw 'Set MLC to the installation directory before deploying.' }
$installationRoot = [IO.Path]::GetFullPath($env:MLC)
if ($installationRoot.TrimEnd('\', '/') -eq $repositoryRoot.TrimEnd('\', '/')) { throw 'MLC must point outside the source checkout.' }
foreach ($directory in @('packages', 'tools', 'runtimes')) {
    $inputRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot $directory)).TrimEnd('\', '/')
    if ($installationRoot -eq $inputRoot -or $installationRoot.StartsWith($inputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "MLC cannot be inside deployment input directory $inputRoot"
    }
}
$Profile = $Profile.ToLowerInvariant()
$targetRoot = Join-Path $repositoryRoot 'target'
if ($env:CARGO_TARGET_DIR) {
    $targetRoot = if ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) { $env:CARGO_TARGET_DIR } else { Join-Path $repositoryRoot $env:CARGO_TARGET_DIR }
}
$artifactRoot = Join-Path $targetRoot $Profile
if (!$SkipBuild) {
    Push-Location $repositoryRoot
    try {
        $buildArguments = @('build', '--bins')
        if ($Profile -eq 'release') { $buildArguments += '--release' }
        & cargo @buildArguments
        if ($LASTEXITCODE -ne 0) { throw "Cargo build failed ($LASTEXITCODE)." }
    } finally { Pop-Location }
}
function Require-File([string]$Path) {
    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Required deployment input not found: $Path" }
    return [IO.Path]::GetFullPath($Path)
}
function Find-File([string[]]$Candidates) {
    foreach ($candidate in $Candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate -PathType Leaf)) { return [IO.Path]::GetFullPath($candidate) }
    }
    return $null
}
# Preserve unrelated [[Packages]] records verbatim; replace records shipped by this checkout.
function Read-PackageRecords([string]$Path) {
    $records = [ordered]@{}
    if (!(Test-Path -LiteralPath $Path)) { return ,$records }
    $content = [IO.File]::ReadAllText($Path)
    $headers = [regex]::Matches($content, '(?m)^\s*\[\[Packages\]\][ \t]*(?:#[^\r\n]*)?\r?$')
    if ($headers.Count -eq 0) {
        if ($content -match '^\s*(?:Packages\s*=\s*\[\s*\]\s*)?$') { return ,$records }
        throw "Cannot read package records in $Path"
    }
    for ($index = 0; $index -lt $headers.Count; $index++) {
        $start = $headers[$index].Index
        $end = if ($index + 1 -lt $headers.Count) { $headers[$index + 1].Index } else { $content.Length }
        $record = $content.Substring($start, $end - $start).Trim()
        $metadata = [regex]::Split($record.Substring($record.IndexOf(']]') + 2), '(?m)^\s*\[')[0]
        $name = [regex]::Match($metadata, '(?m)^\s*(?:Name|"Name"|''Name'')\s*=\s*["'']([A-Za-z0-9_-]+)["''][ \t]*(?:#[^\r\n]*)?\r?$')
        $version = [regex]::Match($metadata, '(?m)^\s*(?:Version|"Version"|''Version'')\s*=\s*["'']([0-9]+\.[0-9]+\.[0-9]+)["''][ \t]*(?:#[^\r\n]*)?\r?$')
        if (!$name.Success -or !$version.Success) { throw "Invalid package Name/Version in $Path" }
        $key = $name.Groups[1].Value + '/' + $version.Groups[1].Value
        if ($records.Contains($key)) { throw "Duplicate package $key in $Path" }
        $records[$key] = $record
    }
    return ,$records
}
function Copy-Tree([string]$Source, [string]$Destination, [switch]$PackageSources) {
    if (!(Test-Path -LiteralPath $Source -PathType Container)) { return }
    [IO.Directory]::CreateDirectory($Destination) | Out-Null
    foreach ($entry in Get-ChildItem -LiteralPath $Source -Force) {
        if ($PackageSources -and $entry.Name -in @('.compiled', 'packages.toml', 'packages.toml.lock')) { continue }
        if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Deployment inputs cannot be symlinks: $($entry.FullName)" }
        $to = Join-Path $Destination $entry.Name
        if ($entry.PSIsContainer) { Copy-Tree $entry.FullName $to -PackageSources:$PackageSources }
        else { Copy-Item -LiteralPath $entry.FullName -Destination $to -Force }
    }
}
function Publish-File([string]$Source, [string]$Destination) {
    $staged = Join-Path $installationRoot ('.deploy-' + [guid]::NewGuid().ToString('N'))
    Copy-Item -LiteralPath $Source -Destination $staged
    $backup = $null
    try {
        if (Test-Path -LiteralPath $Destination) {
            $backupDirectory = Join-Path $installationRoot '.deploy'
            [IO.Directory]::CreateDirectory($backupDirectory) | Out-Null
            $backup = Join-Path $backupDirectory ([guid]::NewGuid().ToString('N') + '-' + [IO.Path]::GetFileName($Destination))
            Move-Item -LiteralPath $Destination -Destination $backup
        }
        Move-Item -LiteralPath $staged -Destination $Destination
    } catch {
        if ($backup -and !(Test-Path -LiteralPath $Destination)) { Move-Item -LiteralPath $backup -Destination $Destination }
        throw
    } finally { if (Test-Path -LiteralPath $staged) { Remove-Item -LiteralPath $staged } }
    if ($backup) {
        try { Remove-Item -LiteralPath $backup -ErrorAction Stop }
        catch { Write-Host "Previous executable is still running; restart its process to release $backup" }
    }
}
$programs = @('mlc.exe', 'mlc-build.exe', 'mlc-fmt.exe', 'mlc-lsp.exe')
foreach ($program in $programs) { Require-File (Join-Path $artifactRoot $program) | Out-Null }
$sourceCatalogue = Require-File (Join-Path $repositoryRoot 'packages/packages.toml')
$destinationCatalogue = Join-Path $installationRoot 'packages/packages.toml'
$sourceRecords = Read-PackageRecords $sourceCatalogue
# Resolve every required input before changing the installation.
$llvmBins = @()
if ($env:LLVM_SYS_231_PREFIX) { $llvmBins += Join-Path $env:LLVM_SYS_231_PREFIX 'bin' }
if ($env:SCOOP) { $llvmBins += Join-Path $env:SCOOP 'apps/llvm-dev/current/bin' }
$llvmCommand = Get-Command 'llvm-ar.exe' -ErrorAction SilentlyContinue
if ($llvmCommand) { $llvmBins += Split-Path -Parent $llvmCommand.Source }
$llvmDll = Find-File (@((Join-Path $artifactRoot 'LLVM-C.dll')) + @($llvmBins | ForEach-Object { Join-Path $_ 'LLVM-C.dll' }))
if (!$llvmDll) { throw 'LLVM-C.dll not found in build output or the configured LLVM installation.' }
$toolFiles = [ordered]@{}
foreach ($tool in @('lld-link.exe', 'ld.lld.exe', 'llvm-ar.exe')) {
    $toolFiles[$tool] = Find-File (@((Join-Path $repositoryRoot "tools/$tool")) + @($llvmBins | ForEach-Object { Join-Path $_ $tool }) + @((Join-Path $installationRoot "tools/$tool")))
}
foreach ($tool in @('lld-link.exe', 'llvm-ar.exe')) { if (!$toolFiles[$tool]) { throw "Required native tool not found: $tool" } }
foreach ($library in @('msvcrt.lib', 'vcruntime.lib', 'ucrt.lib', 'kernel32.lib', 'legacy_stdio_definitions.lib', 'oldnames.lib')) {
    if (!(Find-File @((Join-Path $repositoryRoot "runtimes/x86_64-pc-windows-msvc/$library"), (Join-Path $installationRoot "runtimes/x86_64-pc-windows-msvc/$library")))) { throw "Required MSVC runtime library not found: $library" }
}
[IO.Directory]::CreateDirectory((Join-Path $installationRoot 'packages')) | Out-Null
$lockPath = Join-Path $installationRoot 'packages/packages.toml.lock'
$catalogueLock = [IO.File]::Open($lockPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
try {
    $records = Read-PackageRecords $destinationCatalogue
    foreach ($key in $sourceRecords.Keys) { $records[$key] = $sourceRecords[$key] }
    foreach ($program in $programs) { Publish-File (Join-Path $artifactRoot $program) (Join-Path $installationRoot $program) }
    Publish-File $llvmDll (Join-Path $installationRoot 'LLVM-C.dll')
    foreach ($pdb in @('mlc.pdb', 'mlc_build.pdb', 'mlc_fmt.pdb', 'mlc_lsp.pdb')) {
        $source = Join-Path $artifactRoot $pdb
        $destination = Join-Path $installationRoot $pdb
        if (Test-Path -LiteralPath $source) { Publish-File $source $destination }
        elseif (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination }
    }
    [IO.Directory]::CreateDirectory((Join-Path $installationRoot 'tools')) | Out-Null
    foreach ($tool in $toolFiles.Keys) {
        if ($toolFiles[$tool] -and [IO.Path]::GetFullPath($toolFiles[$tool]) -ne [IO.Path]::GetFullPath((Join-Path $installationRoot "tools/$tool"))) {
            Copy-Item -LiteralPath $toolFiles[$tool] -Destination (Join-Path $installationRoot "tools/$tool") -Force
        }
    }
    Copy-Tree (Join-Path $repositoryRoot 'runtimes') (Join-Path $installationRoot 'runtimes')
    foreach ($key in $sourceRecords.Keys) {
        $packageRoot = [IO.Path]::GetFullPath((Join-Path $installationRoot ("packages/" + $key)))
        $sourceRoot = Join-Path $repositoryRoot ("packages/" + $key + '/source')
        if (!(Test-Path -LiteralPath $sourceRoot -PathType Container)) { continue }
        if ($key -like 'std/*' -and !(Test-Path -LiteralPath (Join-Path $sourceRoot 'mem.toml'))) {
            $legacyMem = Join-Path $packageRoot 'source/mem.toml'
            if (Test-Path -LiteralPath $legacyMem -PathType Leaf) { Remove-Item -LiteralPath $legacyMem }
            $declarations = Join-Path $packageRoot 'source/.compiled/declarations'
            if (Test-Path -LiteralPath $declarations -PathType Container) {
                foreach ($target in Get-ChildItem -LiteralPath $declarations -Directory) {
                    $legacyMemSymbols = Join-Path $target.FullName 'std/mem.sym'
                    if (Test-Path -LiteralPath $legacyMemSymbols -PathType Leaf) { Remove-Item -LiteralPath $legacyMemSymbols }
                }
            }
        }
        foreach ($mode in @('debug', 'release')) {
            $obsoleteSource = [IO.Path]::GetFullPath((Join-Path $packageRoot "$mode/source"))
            if (!$obsoleteSource.StartsWith($packageRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected obsolete source directory' }
            if (Test-Path -LiteralPath $obsoleteSource) { Remove-Item -LiteralPath $obsoleteSource -Recurse -Force }
            $modeDirectory = Join-Path $packageRoot $mode
            if ((Test-Path -LiteralPath $modeDirectory) -and !(Get-ChildItem -LiteralPath $modeDirectory -Force | Select-Object -First 1)) { Remove-Item -LiteralPath $modeDirectory }
        }
    }
    Copy-Tree (Join-Path $repositoryRoot 'packages') (Join-Path $installationRoot 'packages') -PackageSources
    $temporaryCatalogue = Join-Path $installationRoot ('packages/.catalogue-' + [guid]::NewGuid().ToString('N') + '.toml')
    try {
        [IO.File]::WriteAllText($temporaryCatalogue, ($records.Values -join ([Environment]::NewLine + [Environment]::NewLine)) + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
        [IO.File]::Move($temporaryCatalogue, $destinationCatalogue, $true)
    } finally { if (Test-Path -LiteralPath $temporaryCatalogue) { Remove-Item -LiteralPath $temporaryCatalogue } }
    & (Join-Path $installationRoot 'mlc.exe') setup
    if ($LASTEXITCODE -ne 0) { throw "Package setup failed ($LASTEXITCODE)." }
    Write-Host "Deployed $Profile to $installationRoot"
} finally {
    $catalogueLock.Dispose()
    Remove-Item -LiteralPath $lockPath
    $backupDirectory = Join-Path $installationRoot '.deploy'
    if ((Test-Path -LiteralPath $backupDirectory) -and !(Get-ChildItem -LiteralPath $backupDirectory -Force | Select-Object -First 1)) { Remove-Item -LiteralPath $backupDirectory }
}
