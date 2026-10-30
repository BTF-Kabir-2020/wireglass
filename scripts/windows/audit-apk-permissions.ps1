#Requires -Version 5.1
param(
    [Parameter(Mandatory = $true)][string]$Apk
)
$ErrorActionPreference = "Stop"
if (-not (Test-Path $Apk)) { throw "APK not found: $Apk" }

$aapt = $env:AAPT
if (-not $aapt) {
    if (-not $env:ANDROID_HOME) { throw "set ANDROID_HOME or AAPT" }
    $aapt = Get-ChildItem -Path (Join-Path $env:ANDROID_HOME "build-tools") -Filter aapt.exe -Recurse |
        Sort-Object FullName | Select-Object -Last 1 -ExpandProperty FullName
}
if (-not $aapt) { throw "aapt.exe not found" }

$allow = @("android.permission.INTERNET")
$dump = & $aapt dump xmltree $Apk AndroidManifest.xml
$found = @()
foreach ($line in $dump) {
    if ($line -match 'android:name.*=.*"(android\.permission\.[^"]+)"') {
        $found += $Matches[1]
    }
}
$found = $found | Select-Object -Unique

Write-Host "APK=$Apk"
Write-Host "Found permissions:"
if (-not $found -or $found.Count -eq 0) {
    Write-Host "  (none)"
    throw "FAIL: expected at least INTERNET"
}
$bad = $false
$hasInternet = $false
foreach ($p in $found) {
    Write-Host "  - $p"
    if ($p -eq "android.permission.INTERNET") { $hasInternet = $true }
    if ($allow -notcontains $p) {
        Write-Host "UNEXPECTED: $p"
        $bad = $true
    }
}
if (-not $hasInternet) { throw "FAIL: missing INTERNET" }
if ($bad) { throw "FAIL: unexpected permissions" }
Write-Host "OK: permissions allow-list satisfied"
