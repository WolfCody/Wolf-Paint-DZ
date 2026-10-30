# Windows PowerShell version of generate-wrapper.sh (same two stages and the same verification).
# Needs any installed Gradle (>= 8) and Java on PATH, plus network access to services.gradle.org.
# NOTE: written for PowerShell 5.1+/7 but NOT executed by the author (no PowerShell available);
# scripts/generate-wrapper.sh is the tested path.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

function Invoke-Checked([string]$what, [scriptblock]$block) {
    & $block
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE)" }
}

$props = Get-Content "gradle/wrapper/gradle-wrapper.properties" -Raw
if ($props -notmatch 'gradle-([0-9][0-9.]*)-bin\.zip') { throw "cannot read the Gradle version from gradle-wrapper.properties" }
$version = $Matches[1]
if (-not (Get-Command gradle -ErrorAction SilentlyContinue)) { throw "no 'gradle' on PATH (any Gradle >= 8 works for stage 1)" }

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("rpx-wrapper-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
New-Item -ItemType File (Join-Path $tmp "settings.gradle") | Out-Null
$repo = (Get-Location).Path
try {
    Push-Location $tmp

    Write-Host "== stage 1: gradle wrapper --gradle-version $version"
    Invoke-Checked "gradle wrapper (stage 1)" { gradle wrapper --gradle-version $version --distribution-type bin }
    if (-not (Test-Path "gradlew.bat") -or -not (Test-Path "gradle/wrapper/gradle-wrapper.jar")) { throw "stage 1 did not produce the wrapper" }

    Write-Host "== stage 2: .\gradlew.bat wrapper (regenerates the JAR with Gradle $version)"
    Invoke-Checked "gradlew wrapper (stage 2)" { & .\gradlew.bat wrapper --gradle-version $version --distribution-type bin }

    Write-Host "== verification: .\gradlew.bat --version"
    $reported = & .\gradlew.bat --version
    if ($LASTEXITCODE -ne 0) { throw "gradlew --version failed" }
    $reported | ForEach-Object { Write-Host $_ }
    if (-not ($reported | Where-Object { $_ -match ("^Gradle " + [regex]::Escape($version) + "\s*$") })) {
        throw "generated wrapper does not run Gradle $version"
    }
    if ((Get-Content "gradle/wrapper/gradle-wrapper.properties" -Raw) -notmatch [regex]::Escape("gradle-$version-bin.zip")) {
        throw "generated gradle-wrapper.properties does not point to Gradle $version"
    }

    Copy-Item "gradlew", "gradlew.bat" $repo
    New-Item -ItemType Directory -Force (Join-Path $repo "gradle/wrapper") | Out-Null
    Copy-Item "gradle/wrapper/gradle-wrapper.jar" (Join-Path $repo "gradle/wrapper")
    Pop-Location

    $sha = (Get-FileHash (Join-Path $repo "gradle/wrapper/gradle-wrapper.jar") -Algorithm SHA256).Hash.ToLower()
    Write-Host "Wrapper for Gradle $version written."
    Write-Host "gradle-wrapper.jar sha256: $sha"
    Write-Host "(compare with the 'Wrapper JAR Checksum' of Gradle $version on https://gradle.org/release-checksums/)"
    Write-Host "Commit gradlew, gradlew.bat, gradle/wrapper/gradle-wrapper.jar."
} finally {
    Set-Location $repo
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
