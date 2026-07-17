$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Repository = 'HawkItzme/ContextDroid'
$Asset = 'contextdroid-x86_64-pc-windows-msvc.zip'
$Version = if ($env:CONTEXTDROID_VERSION) {
    $env:CONTEXTDROID_VERSION
} else {
    'v0.1.0-alpha.1'
}
$InstallDir = if ($env:CONTEXTDROID_INSTALL_DIR) {
    $env:CONTEXTDROID_INSTALL_DIR
} elseif ($env:LOCALAPPDATA) {
    Join-Path $env:LOCALAPPDATA 'ContextDroid\bin'
} else {
    Join-Path $HOME '.local\bin'
}
$ReleaseBase = if ($env:CONTEXTDROID_RELEASE_BASE) {
    $env:CONTEXTDROID_RELEASE_BASE.TrimEnd('/', '\')
} else {
    "https://github.com/$Repository/releases/download/$Version"
}

if ($Version -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
    throw 'ContextDroid install error: invalid CONTEXTDROID_VERSION'
}
if (-not [Environment]::Is64BitOperatingSystem) {
    throw 'ContextDroid install error: unsupported Windows architecture'
}

function Copy-ReleaseFile {
    param(
        [Parameter(Mandatory)][string] $Name,
        [Parameter(Mandatory)][string] $Destination
    )

    if ($ReleaseBase.StartsWith('file://', [StringComparison]::OrdinalIgnoreCase)) {
        $sourceRoot = ([Uri]$ReleaseBase).LocalPath
        Copy-Item -LiteralPath (Join-Path $sourceRoot $Name) -Destination $Destination
    } elseif (Test-Path -LiteralPath $ReleaseBase -PathType Container) {
        Copy-Item -LiteralPath (Join-Path $ReleaseBase $Name) -Destination $Destination
    } else {
        Invoke-WebRequest -UseBasicParsing -Uri "$ReleaseBase/$Name" -OutFile $Destination
    }
}

$TempDir = Join-Path ([IO.Path]::GetTempPath()) ("contextdroid-install-" + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($TempDir) | Out-Null

try {
    $Archive = Join-Path $TempDir $Asset
    $Checksums = Join-Path $TempDir 'SHA256SUMS'
    Copy-ReleaseFile -Name $Asset -Destination $Archive
    Copy-ReleaseFile -Name 'SHA256SUMS' -Destination $Checksums

    $checksumLine = Get-Content -LiteralPath $Checksums | Where-Object {
        $_ -match ('^[0-9A-Fa-f]{64}\s+\*?' + [Regex]::Escape($Asset) + '$')
    } | Select-Object -First 1
    if (-not $checksumLine) {
        throw 'ContextDroid install error: asset checksum is missing'
    }
    $expected = ($checksumLine -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expected -ne $actual) {
        throw 'ContextDroid install error: checksum mismatch'
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $allowed = @('contextdroid.exe', 'LICENSE', 'UPSTREAM.md', 'THIRD_PARTY_NOTICES.md')
        foreach ($entry in $zip.Entries) {
            $segments = $entry.FullName -split '[/\\]'
            if ([IO.Path]::IsPathRooted($entry.FullName) -or $segments -contains '..' -or $entry.FullName -notin $allowed) {
                throw 'ContextDroid install error: unsafe archive path'
            }
        }
        $binaryEntry = $zip.GetEntry('contextdroid.exe')
        if (-not $binaryEntry) {
            throw 'ContextDroid install error: archive does not contain contextdroid.exe'
        }
        $stagedBinary = Join-Path $TempDir 'contextdroid.exe'
        $input = $binaryEntry.Open()
        $output = [IO.File]::Create($stagedBinary)
        try {
            $input.CopyTo($output)
        } finally {
            $output.Dispose()
            $input.Dispose()
        }
    } finally {
        $zip.Dispose()
    }

    $reportedVersion = & $stagedBinary --version
    if ($LASTEXITCODE -ne 0 -or $reportedVersion -notmatch [Regex]::Escape($Version.TrimStart('v'))) {
        throw "ContextDroid install error: downloaded binary version does not match $Version"
    }

    [IO.Directory]::CreateDirectory($InstallDir) | Out-Null
    $destination = Join-Path $InstallDir 'contextdroid.exe'
    $transactionFile = Join-Path $InstallDir ('.contextdroid.new.' + [Guid]::NewGuid().ToString('N'))
    Copy-Item -LiteralPath $stagedBinary -Destination $transactionFile
    Move-Item -LiteralPath $transactionFile -Destination $destination -Force

    if ($env:CONTEXTDROID_NO_PATH_UPDATE -ne '1') {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $parts = @($userPath -split ';' | Where-Object { $_ })
        if (-not ($parts | Where-Object { $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\') })) {
            $newUserPath = (@($parts) + $InstallDir) -join ';'
            [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
        }
        if (-not (($env:Path -split ';') | Where-Object { $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\') })) {
            $env:Path = "$InstallDir;$env:Path"
        }
    }

    & $destination --version
    Write-Host "ContextDroid installed to $destination"
    Write-Host 'Next: contextdroid integrations claude install'
} finally {
    Remove-Item -LiteralPath $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
