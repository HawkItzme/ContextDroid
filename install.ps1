$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:Repository = 'HawkItzme/ContextDroid'
$script:Asset = 'contextdroid-x86_64-pc-windows-msvc.zip'
$script:LatestUrl = "https://github.com/$script:Repository/releases/latest"
$script:AllowedDownloadHosts = @(
    'github.com',
    'objects.githubusercontent.com',
    'release-assets.githubusercontent.com'
)

function Test-ContextDroidVersion {
    param([Parameter(Mandatory)][string] $Version, [switch] $StableOnly)
    $pattern = if ($StableOnly) {
        '^v[0-9]+\.[0-9]+\.[0-9]+$'
    } else {
        '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$'
    }
    return $Version -match $pattern
}

function Test-ContextDroidUri {
    param(
        [Parameter(Mandatory)][Uri] $Uri,
        [Parameter(Mandatory)][string[]] $AllowedHosts
    )
    if ($Uri.Scheme -ne 'https' -or $Uri.UserInfo -or $Uri.Port -ne 443) {
        throw "ContextDroid install error: unsafe release URL $Uri"
    }
    if ($Uri.DnsSafeHost -notin $AllowedHosts) {
        throw "ContextDroid install error: unexpected release host $($Uri.DnsSafeHost)"
    }
}

function Invoke-ContextDroidDownload {
    param(
        [Parameter(Mandatory)][Uri] $Uri,
        [string] $OutFile,
        [Parameter(Mandatory)][string[]] $AllowedHosts
    )
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $client = [Net.Http.HttpClient]::new($handler)
    $client.Timeout = [TimeSpan]::FromMinutes(3)
    try {
        $current = $Uri
        for ($redirects = 0; $redirects -le 5; $redirects++) {
            Test-ContextDroidUri -Uri $current -AllowedHosts $AllowedHosts
            $response = $client.GetAsync($current).GetAwaiter().GetResult()
            if ([int]$response.StatusCode -ge 300 -and [int]$response.StatusCode -lt 400) {
                if (-not $response.Headers.Location -or $redirects -eq 5) {
                    throw 'ContextDroid install error: invalid or excessive release redirect'
                }
                $current = if ($response.Headers.Location.IsAbsoluteUri) {
                    $response.Headers.Location
                } else {
                    [Uri]::new($current, $response.Headers.Location)
                }
                $response.Dispose()
                continue
            }
            if (-not $response.IsSuccessStatusCode) {
                throw "ContextDroid install error: download failed with HTTP $([int]$response.StatusCode)"
            }
            if ($OutFile) {
                $input = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
                $output = [IO.File]::Create($OutFile)
                try {
                    $input.CopyTo($output)
                } finally {
                    $output.Dispose()
                    $input.Dispose()
                }
            }
            return $current
        }
        throw 'ContextDroid install error: MaximumRedirection exceeded'
    } finally {
        $client.Dispose()
        $handler.Dispose()
    }
}

function Resolve-ContextDroidVersion {
    param([string] $RequestedVersion, [string] $CustomBase)
    if ($CustomBase -and -not $RequestedVersion) {
        throw 'ContextDroid install error: custom release base requires CONTEXTDROID_VERSION'
    }
    if ($RequestedVersion) {
        if (-not (Test-ContextDroidVersion -Version $RequestedVersion)) {
            throw 'ContextDroid install error: invalid CONTEXTDROID_VERSION'
        }
        return $RequestedVersion
    }
    $effective = Invoke-ContextDroidDownload -Uri $script:LatestUrl -AllowedHosts @('github.com')
    $prefix = "https://github.com/$script:Repository/releases/tag/"
    if (-not $effective.AbsoluteUri.StartsWith($prefix, [StringComparison]::Ordinal)) {
        throw 'ContextDroid install error: latest release redirected to an unexpected URL'
    }
    $resolved = $effective.AbsoluteUri.Substring($prefix.Length).TrimEnd('/')
    if (-not (Test-ContextDroidVersion -Version $resolved -StableOnly)) {
        throw 'ContextDroid install error: latest release is not a stable semantic version'
    }
    return $resolved
}

function Copy-ContextDroidReleaseFile {
    param(
        [Parameter(Mandatory)][string] $ReleaseBase,
        [Parameter(Mandatory)][string] $Name,
        [Parameter(Mandatory)][string] $Destination
    )
    if ($ReleaseBase.StartsWith('file://', [StringComparison]::OrdinalIgnoreCase)) {
        Copy-Item -LiteralPath (Join-Path ([Uri]$ReleaseBase).LocalPath $Name) -Destination $Destination
    } elseif (Test-Path -LiteralPath $ReleaseBase -PathType Container) {
        Copy-Item -LiteralPath (Join-Path $ReleaseBase $Name) -Destination $Destination
    } else {
        $uri = [Uri]::new("$($ReleaseBase.TrimEnd('/', '\'))/$Name")
        Invoke-ContextDroidDownload -Uri $uri -OutFile $Destination -AllowedHosts $script:AllowedDownloadHosts | Out-Null
    }
}

function Install-ContextDroid {
    $requestedVersion = $env:CONTEXTDROID_VERSION
    $customBase = $env:CONTEXTDROID_RELEASE_BASE
    $version = Resolve-ContextDroidVersion -RequestedVersion $requestedVersion -CustomBase $customBase
    $releaseBase = if ($customBase) {
        $customBase.TrimEnd('/', '\')
    } else {
        "https://github.com/$script:Repository/releases/download/$version"
    }
    $installDir = if ($env:CONTEXTDROID_INSTALL_DIR) {
        $env:CONTEXTDROID_INSTALL_DIR
    } elseif ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA 'ContextDroid\bin'
    } else {
        Join-Path $HOME '.local\bin'
    }
    if (-not [Environment]::Is64BitOperatingSystem) {
        throw 'ContextDroid install error: unsupported Windows architecture'
    }

    $tempDir = Join-Path ([IO.Path]::GetTempPath()) ("contextdroid-install-" + [Guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory($tempDir) | Out-Null
    $destination = Join-Path $installDir 'contextdroid.exe'
    $backup = $null
    $pathChanged = $false
    $previousUserPath = $null
    $previousProcessPath = $env:Path
    $committed = $false
    try {
        $archive = Join-Path $tempDir $script:Asset
        $checksums = Join-Path $tempDir 'SHA256SUMS'
        Copy-ContextDroidReleaseFile -ReleaseBase $releaseBase -Name $script:Asset -Destination $archive
        Copy-ContextDroidReleaseFile -ReleaseBase $releaseBase -Name 'SHA256SUMS' -Destination $checksums

        $checksumLine = Get-Content -LiteralPath $checksums | Where-Object {
            $_ -match ('^[0-9A-Fa-f]{64}\s+\*?' + [Regex]::Escape($script:Asset) + '$')
        } | Select-Object -First 1
        if (-not $checksumLine) {
            throw 'ContextDroid install error: asset checksum is missing'
        }
        $expected = ($checksumLine -split '\s+')[0].ToLowerInvariant()
        $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($expected -ne $actual) {
            throw 'ContextDroid install error: checksum mismatch'
        }

        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [IO.Compression.ZipFile]::OpenRead($archive)
        try {
            $allowed = @('contextdroid.exe', 'LICENSE', 'UPSTREAM.md', 'THIRD_PARTY_NOTICES.md')
            foreach ($entry in $zip.Entries) {
                $segments = $entry.FullName -split '[/\\]'
                $unixType = ($entry.ExternalAttributes -shr 16) -band 0xF000
                if ([IO.Path]::IsPathRooted($entry.FullName) -or
                    $segments -contains '..' -or
                    $entry.FullName -notin $allowed -or
                    $unixType -eq 0xA000) {
                    throw 'ContextDroid install error: unsafe archive path'
                }
            }
            $binaryEntries = @($zip.Entries | Where-Object FullName -eq 'contextdroid.exe')
            if ($binaryEntries.Count -ne 1) {
                throw 'ContextDroid install error: archive does not contain exactly one contextdroid.exe'
            }
            $stagedBinary = Join-Path $tempDir 'contextdroid.exe'
            $input = $binaryEntries[0].Open()
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
        if ($LASTEXITCODE -ne 0 -or $reportedVersion -notmatch ('^contextdroid ' + [Regex]::Escape($version.TrimStart('v')) + '(?:\s|$)')) {
            throw "ContextDroid install error: downloaded binary version does not match $version"
        }

        [IO.Directory]::CreateDirectory($installDir) | Out-Null
        $transactionFile = Join-Path $installDir ('.contextdroid.new.' + [Guid]::NewGuid().ToString('N'))
        Copy-Item -LiteralPath $stagedBinary -Destination $transactionFile
        if (Test-Path -LiteralPath $destination) {
            $backup = Join-Path $installDir ('.contextdroid.backup.' + [Guid]::NewGuid().ToString('N'))
            Move-Item -LiteralPath $destination -Destination $backup
        }
        Move-Item -LiteralPath $transactionFile -Destination $destination

        if ($env:CONTEXTDROID_NO_PATH_UPDATE -ne '1') {
            $previousUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
            $parts = @($previousUserPath -split ';' | Where-Object { $_ })
            if (-not ($parts | Where-Object { $_.TrimEnd('\') -ieq $installDir.TrimEnd('\') })) {
                [Environment]::SetEnvironmentVariable('Path', ((@($parts) + $installDir) -join ';'), 'User')
                $pathChanged = $true
            }
            if (-not (($env:Path -split ';') | Where-Object { $_.TrimEnd('\') -ieq $installDir.TrimEnd('\') })) {
                $env:Path = "$installDir;$env:Path"
            }
        }

        & $destination --version
        if ($LASTEXITCODE -ne 0) {
            throw 'ContextDroid install error: installed binary failed verification'
        }
        $committed = $true
        if ($backup) {
            Remove-Item -LiteralPath $backup -Force
            $backup = $null
        }
        Write-Host "ContextDroid installed to $destination"
        Write-Host 'Next: contextdroid setup detect'
    } finally {
        if (-not $committed) {
            Remove-Item -LiteralPath $destination -Force -ErrorAction SilentlyContinue
            if ($backup -and (Test-Path -LiteralPath $backup)) {
                Move-Item -LiteralPath $backup -Destination $destination
            }
            if ($pathChanged) {
                [Environment]::SetEnvironmentVariable('Path', $previousUserPath, 'User')
            }
            $env:Path = $previousProcessPath
        }
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($env:CONTEXTDROID_INSTALLER_TEST -ne '1') {
    Install-ContextDroid
}
