param([Parameter(Mandatory=$true)][string]$Source)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$sourceRoot = (Resolve-Path -LiteralPath $Source).Path
$fixture = Join-Path $root '.local-tests/p5js'
$proof = Join-Path $root 'docs/proofs/p5-midi'
foreach ($required in @('src/objects/midi-pattern-grid.object.ts','src/midi/smf.ts','scripts/create-demo-midi.mjs','tests')) {
    if (-not (Test-Path -LiteralPath (Join-Path $sourceRoot $required))) { throw "Missing supplied test file: $required" }
}
New-Item -ItemType Directory -Force -Path $fixture,(Join-Path $fixture 'scripts'),(Join-Path $fixture 'public'),$proof | Out-Null
$records = @()
foreach ($directory in @('src','tests','scripts')) {
    $sourceDirectory = Join-Path $sourceRoot $directory
    foreach ($file in Get-ChildItem -LiteralPath $sourceDirectory -File -Recurse) {
        $records += [pscustomobject]@{path=$file.FullName;sha256=(Get-FileHash -LiteralPath $file.FullName).Hash}
    }
}
$existingMidi = Join-Path $sourceRoot 'public/song.mid'
if (Test-Path -LiteralPath $existingMidi) { $records += [pscustomobject]@{path=$existingMidi;sha256=(Get-FileHash -LiteralPath $existingMidi).Hash} }
$records | ConvertTo-Json | Set-Content (Join-Path $proof 'source-hashes.json') -Encoding UTF8
Copy-Item -LiteralPath (Join-Path $sourceRoot 'src'),(Join-Path $sourceRoot 'tests') -Destination $fixture -Recurse -Force
Copy-Item -LiteralPath (Join-Path $sourceRoot 'scripts/create-demo-midi.mjs') -Destination (Join-Path $fixture 'scripts') -Force
$package = @'
{
  "name": "web-render-p5-midi-test", "private": true, "type": "module",
  "scripts": { "test": "node --test tests/*.test.ts" },
  "dependencies": { "web-render": "file:../../packages/vi5", "p5": "2.2.1" }
}
'@
$config = @'
import { defineConfig } from "web-render/config";
import { fileURLToPath } from "node:url";
export default defineConfig({
  name: "p5-midi-test",
  vitePlugins: [{name: "p5-midi-vi5-api-compatibility", config: () => ({resolve: {alias: {
    vi5: fileURLToPath(new URL("../../packages/vi5/src/index.ts", import.meta.url)),
    p5: fileURLToPath(new URL("./node_modules/p5/dist/app.js", import.meta.url)),
  }}})}],
});
'@
[IO.File]::WriteAllText((Join-Path $fixture 'package.json'), $package, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $fixture 'web-render.config.ts'), $config, [Text.UTF8Encoding]::new($false))
& node (Join-Path $fixture 'scripts/create-demo-midi.mjs')
if ($LASTEXITCODE) { throw 'Supplied MIDI generator failed' }
Copy-Item -LiteralPath (Join-Path $fixture 'public/song.mid') -Destination (Join-Path $proof 'generated-song.mid') -Force
& npm install --prefix $fixture
if ($LASTEXITCODE) { throw 'Fixture dependency installation failed' }
& node (Join-Path $root 'scripts/validate-p5-midi.mjs') $fixture $proof
if ($LASTEXITCODE) { throw 'Generated MIDI validation failed' }
Write-Output "Prepared test copy: $fixture"
