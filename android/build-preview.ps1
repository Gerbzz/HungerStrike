param(
    [string]$SdkRoot = $env:ANDROID_HOME,
    [string]$NdkRoot = '',
    [string]$JavaRoot = 'C:\Program Files\Android\Android Studio\jbr',
    [ValidateSet('aarch64-linux-android', 'x86_64-linux-android')]
    [string]$Target = 'aarch64-linux-android',
    [ValidateSet('Debug', 'Release')]
    [string]$BuildProfile = 'Debug',
    [string]$OutputName = ''
)

$ErrorActionPreference = 'Stop'
if (-not $SdkRoot) { throw 'ANDROID_HOME or -SdkRoot is required' }
if (-not $NdkRoot) { $NdkRoot = Join-Path $SdkRoot 'ndk\26.3.11579264' }
if (-not $env:HUNGERSTRIKE_DEBUG_KEYSTORE_PASSWORD) { throw 'HUNGERSTRIKE_DEBUG_KEYSTORE_PASSWORD is required for the local debug keystore' }
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$tools = Join-Path $SdkRoot 'build-tools\35.0.1'
$aapt = Join-Path $tools 'aapt.exe'
$zipalign = Join-Path $tools 'zipalign.exe'
$apksigner = Join-Path $tools 'apksigner.bat'
$strip = Join-Path $NdkRoot 'toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-strip.exe'
$androidJar = Join-Path $SdkRoot 'platforms\android-33\android.jar'
$keystore = Join-Path $env:USERPROFILE '.android\debug.keystore'
foreach ($required in @($aapt, $zipalign, $apksigner, $strip, $androidJar, $keystore)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Missing build input: $required" }
}

$env:JAVA_HOME = $JavaRoot
$env:ANDROID_NDK_HOME = $NdkRoot
$profileDir = $BuildProfile.ToLowerInvariant()
$abi = if ($Target -eq 'aarch64-linux-android') { 'arm64-v8a' } else { 'x86_64' }
if (-not $OutputName) {
    $OutputName = if ($abi -eq 'arm64-v8a' -and $BuildProfile -eq 'Debug') {
        'HungerStrike-preview.apk'
    } elseif ($abi -eq 'arm64-v8a') {
        'HungerStrike-arm64-release-experimental.apk'
    } else {
        'HungerStrike-x86_64-diagnostic.apk'
    }
}
if ($OutputName -cnotmatch '^[A-Za-z0-9][A-Za-z0-9_.-]*\.apk$') { throw 'OutputName must be a simple .apk filename' }
$apkDir = Join-Path $repo "target\$profileDir\apk"
$intermediate = Join-Path $apkDir 'HungerStrike-intermediate.apk'
$unsigned = Join-Path $apkDir 'HungerStrike-private-unsigned.apk'
$aligned = Join-Path $apkDir 'HungerStrike-private-aligned.apk'
$native = Join-Path $repo "target\$Target\$profileDir\libhungerstrike_android.so"
$stripped = Join-Path $apkDir "libhungerstrike_android-$abi-stripped.so"
$outputDir = Join-Path $repo 'dist\android'
$output = Join-Path $outputDir $OutputName

Push-Location $repo
try {
    $cargoArgs = @('apk', 'build', '-p', 'hungerstrike-android', '--target', $Target)
    if ($BuildProfile -eq 'Release') {
        $cargoArgs += '--release'
        # This remains a local preview, including optimized builds. cargo-apk
        # requires an explicit signing key outside the dev Cargo profile.
        $env:CARGO_APK_RELEASE_KEYSTORE = $keystore
        $env:CARGO_APK_RELEASE_KEYSTORE_PASSWORD = $env:HUNGERSTRIKE_DEBUG_KEYSTORE_PASSWORD
    }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo apk build failed: $LASTEXITCODE" }
    & $strip --strip-all -o $stripped $native
    if ($LASTEXITCODE -ne 0) { throw "llvm-strip failed: $LASTEXITCODE" }

    & $aapt package -f -M 'android\packaging\AndroidManifest.xml' -S 'android\packaging\res' -I $androidJar -F $unsigned
    if ($LASTEXITCODE -ne 0) { throw "aapt package failed: $LASTEXITCODE" }

    $manifest = (& $aapt dump xmltree $unsigned AndroidManifest.xml) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $manifest -notmatch 'android:allowBackup.*0x0' -or
        $manifest -notmatch 'android:usesCleartextTraffic.*0x0' -or
        $manifest -notmatch 'android:dataExtractionRules') {
        throw 'Compiled manifest does not enforce the local-only backup policy'
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $source = [IO.Compression.ZipFile]::OpenRead($intermediate)
    try {
        $library = $source.GetEntry("lib/$abi/libhungerstrike_android.so")
        if ($null -eq $library) { throw "cargo-apk did not package the $abi native library" }
        $zip = [IO.Compression.ZipFile]::Open($unsigned, [IO.Compression.ZipArchiveMode]::Update)
        try {
            $entry = $zip.CreateEntry("lib/$abi/libhungerstrike_android.so", [IO.Compression.CompressionLevel]::Optimal)
            $inputStream = [IO.File]::OpenRead($stripped)
            $outputStream = $entry.Open()
            try { $inputStream.CopyTo($outputStream) }
            finally { $outputStream.Dispose(); $inputStream.Dispose() }
        }
        finally { $zip.Dispose() }
    }
    finally { $source.Dispose() }

    & $zipalign -f 4 $unsigned $aligned
    if ($LASTEXITCODE -ne 0) { throw "zipalign failed: $LASTEXITCODE" }
    New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
    & $apksigner sign --ks $keystore --ks-pass env:HUNGERSTRIKE_DEBUG_KEYSTORE_PASSWORD --out $output $aligned
    if ($LASTEXITCODE -ne 0) { throw "apksigner sign failed: $LASTEXITCODE" }
    & $apksigner verify --verbose $output
    if ($LASTEXITCODE -ne 0) { throw "apksigner verify failed: $LASTEXITCODE" }
    & $zipalign -c 4 $output
    if ($LASTEXITCODE -ne 0) { throw "zipalign check failed: $LASTEXITCODE" }
    $finalManifest = (& $aapt dump xmltree $output AndroidManifest.xml) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $finalManifest -notmatch 'android:allowBackup.*0x0' -or
        $finalManifest -notmatch 'android:dataExtractionRules') {
        throw 'Signed APK lost its backup protection'
    }
    $permissions = (& $aapt dump permissions $output) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $permissions -match 'uses-permission') {
        throw 'Signed APK unexpectedly declares Android permissions'
    }
    Get-FileHash -Algorithm SHA256 $output | Format-List Path,Hash
}
finally { Pop-Location }
