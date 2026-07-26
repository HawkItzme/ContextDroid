$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$env:CONTEXTDROID_INSTALLER_TEST = '1'
. (Join-Path $PSScriptRoot '..\install.ps1')

function Assert-True {
    param([bool] $Condition, [string] $Message)
    if (-not $Condition) {
        throw "FAIL: $Message"
    }
    Write-Host "PASS: $Message"
}

Assert-True (Test-ContextDroidVersion -Version 'v1.2.3' -StableOnly) 'stable tag accepted'
Assert-True (-not (Test-ContextDroidVersion -Version 'v1.2.3-alpha.1' -StableOnly)) 'prerelease excluded from latest stable'
Assert-True (Test-ContextDroidVersion -Version 'v1.2.3-alpha.1') 'explicit prerelease pin accepted'
Assert-True (-not (Test-ContextDroidVersion -Version '../v1.2.3')) 'unsafe version rejected'

try {
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.Dispose()
    Assert-True $true 'System.Net.Http is available in Windows PowerShell 5.1'
} catch {
    throw "FAIL: System.Net.Http is unavailable after loading install.ps1: $($_.Exception.Message)"
}

try {
    Resolve-ContextDroidVersion -RequestedVersion '' -CustomBase 'C:\mirror'
    throw 'FAIL: custom base without explicit version was accepted'
} catch {
    Assert-True ($_.Exception.Message -like '*custom release base requires CONTEXTDROID_VERSION*') 'custom base requires explicit version'
}

foreach ($uri in @('http://github.com/HawkItzme/ContextDroid', 'https://evil.example/release')) {
    try {
        Test-ContextDroidUri -Uri $uri -AllowedHosts @('github.com')
        throw "FAIL: unsafe URI accepted: $uri"
    } catch {
        Assert-True ($_.Exception.Message -like '*unsafe release URL*' -or $_.Exception.Message -like '*unexpected release host*') "unsafe URI rejected: $uri"
    }
}

$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot '..\install.ps1') -Raw
Assert-True ($source.Contains('MaximumRedirection')) 'redirect limit is enforced'
Assert-True ($source.Contains('checksum mismatch')) 'checksum mismatch is fatal'
Assert-True ($source.Contains('contextdroid.backup')) 'binary rollback path is present'
Assert-True ($source.Contains("SetEnvironmentVariable('Path', `$previousUserPath, 'User')")) 'PATH rollback is present'
Assert-True ($source.Contains('$env:Path = $previousProcessPath')) 'process PATH rollback is present'

Write-Host 'All Windows installer contract tests passed'
