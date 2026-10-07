param(
    [string]$CefRuntimeDirectory = $env:CEF_PATH,
    [ValidateSet('debug','release')][string]$Profile = 'release',
    [switch]$SkipBuild,
    [switch]$RuntimeDebug
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    if (-not $SkipBuild) {
        $cargoArgs = @('build','--locked')
        if ($RuntimeDebug -or ($Profile -eq 'debug' -and $env:WEB_RENDER_RUNTIME_DEBUG -eq '1')) {
            if ($Profile -ne 'debug') { throw 'RuntimeDebug is available only for the debug profile.' }
            $cargoArgs += @('--features', 'web-render-aux2/runtime-debug')
        }
        if ($Profile -eq 'release') { $cargoArgs += '--release' }
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
        # Bevy/wgpu native dependencies stay in a separate pinned Cargo workspace.
        & cargo build --locked --release --manifest-path native/web-render-processing-server/Cargo.toml --target-dir native/libprocessing/target
        if ($LASTEXITCODE -ne 0) { throw 'libprocessing worker build failed' }
    }
    $target = Join-Path $root "target/$Profile"
    if (-not $CefRuntimeDirectory) {
        $runtimePath = Join-Path $target 'cef-runtime-path.txt'
        if (Test-Path -LiteralPath $runtimePath) { $CefRuntimeDirectory = (Get-Content -LiteralPath $runtimePath -Raw).Trim() }
    }
    if (-not $CefRuntimeDirectory) { throw 'CEF runtime path is missing; build first or pass -CefRuntimeDirectory.' }
    $archivePath = Join-Path $CefRuntimeDirectory 'archive.json'
    if (-not (Test-Path -LiteralPath $archivePath)) { throw 'CEF archive.json is required to verify the runtime version.' }
    $archive = Get-Content -LiteralPath $archivePath -Raw | ConvertFrom-Json
    if ($archive.name -notlike 'cef_binary_144.0.11+*_windows64_*') { throw "Unexpected CEF runtime: $($archive.name)" }
    foreach ($required in @('libcef.dll','chrome_elf.dll','icudtl.dat','resources.pak','chrome_100_percent.pak','chrome_200_percent.pak','v8_context_snapshot.bin','locales/en-US.pak')) {
        if (-not (Test-Path -LiteralPath (Join-Path $CefRuntimeDirectory $required))) { throw "Missing CEF runtime file: $required" }
    }
    foreach ($required in @('web_render_aux2.dll','web-render-cef-server.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $target $required))) { throw "Missing build artifact: $required" }
    }
    $out = Join-Path $root "dist/windows-$Profile/Plugin/web-render"
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    Copy-Item -LiteralPath (Join-Path $target 'web_render_aux2.dll') -Destination (Join-Path $out 'web-render.aux2') -Force
    Copy-Item -LiteralPath (Join-Path $target 'web-render-cef-server.exe') -Destination $out -Force
    Copy-Item -LiteralPath (Join-Path $root 'native/libprocessing/target/release/web-render-processing-server.exe') -Destination $out -Force
    foreach ($pattern in @('*.dll','*.pak','*.dat','*.bin','*.json')) {
        Get-ChildItem -Path $CefRuntimeDirectory -Filter $pattern -File |
            Where-Object { $_.Name -ne 'web_render_aux2.dll' } | Copy-Item -Destination $out
    }
    $locales = Join-Path $CefRuntimeDirectory 'locales'
    if (Test-Path $locales) { Copy-Item $locales $out -Recurse -Force }
    Copy-Item 'LICENSE' $out
    Copy-Item 'THIRD_PARTY_NOTICES.md' $out
    Copy-Item 'native/libprocessing/LICENSE.md' (Join-Path $out 'libprocessing-LICENSE.md')
    Copy-Item 'native/libprocessing/lygia/LICENSE.md' (Join-Path $out 'lygia-LICENSE.md')
    Copy-Item -LiteralPath (Join-Path $CefRuntimeDirectory 'CREDITS.html') -Destination $out -Force
    # au2 0.10 copies files; keep the default and compatibility configs identical
    # so plain `au2 prepare` / `au2 dev` deploy the complete runtime together.
    $config = [IO.File]::ReadAllText((Join-Path $root 'aviutl2.toml'))
    $begin = '# BEGIN GENERATED RUNTIME ARTIFACTS'
    $end = '# END GENERATED RUNTIME ARTIFACTS'
    $start = $config.IndexOf($begin)
    if ($start -ge 0) {
        $finish = $config.IndexOf($end, $start)
        if ($finish -lt 0) { throw 'Generated runtime artifact block is incomplete.' }
        $config = $config.Remove($start, $finish + $end.Length - $start)
    }
    $config = $config.TrimEnd() + "`n`n$begin`n"
    # CEF may write debug.log next to its executable. Logs are not runtime
    # artifacts and must not change the generated config after a test run.
    $files = Get-ChildItem -LiteralPath $out -File -Recurse |
        Where-Object { $_.Name -ne 'web-render.aux2' -and $_.Extension -ne '.log' } |
        Sort-Object FullName
    $artifactIndex = 0
    foreach ($file in $files) {
        $relative = $file.FullName.Substring($out.Length + 1).Replace('\','/')
        $config += "`n[artifacts.runtime_$artifactIndex]`nplacement_method = 'copy'`ndestination = 'Plugin/web-render/$relative'`n"
        foreach ($runtimeProfile in @('debug','release')) {
            $config += "[artifacts.runtime_$artifactIndex.profiles.$runtimeProfile]`nsource = 'dist/windows-$runtimeProfile/Plugin/web-render/$relative'`n"
        }
        $artifactIndex++
    }
    $config += "`n$end`n"
    [IO.File]::WriteAllText((Join-Path $root 'aviutl2.toml'), $config, [Text.UTF8Encoding]::new($false))
    $configPath = Join-Path $root '.aviutl2-runtime.toml'
    [IO.File]::WriteAllText($configPath, $config, [Text.UTF8Encoding]::new($false))
    Write-Host "Package: $out"
    Write-Host "Development: au2 dev -p $Profile"
} finally { Pop-Location }
