param([string]$Source = '.local-tests/himawari-receiver-upstream')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$sourceRoot = (Resolve-Path -LiteralPath $Source).Path
$fixture = Join-Path $root '.local-tests/himawari-receiver'
$proof = Join-Path $root 'docs/proofs/himawari'
foreach ($required in @('src/objects/drum.object.ts','src/objects/kaiwaiPhrase.object.ts','src/objects/synthSolo.object.ts','src/utils/midi.ts','vi5.config.ts')) {
    if (-not (Test-Path -LiteralPath (Join-Path $sourceRoot $required))) { throw "Missing supplied file: $required" }
}
New-Item -ItemType Directory -Force -Path $fixture,$proof | Out-Null
$records = @()
foreach ($file in Get-ChildItem -LiteralPath (Join-Path $sourceRoot 'src') -File -Recurse) {
    $records += [pscustomobject]@{path=$file.FullName;sha256=(Get-FileHash -LiteralPath $file.FullName).Hash}
}
$records | ConvertTo-Json | Set-Content (Join-Path $proof 'source-hashes.json') -Encoding UTF8
Copy-Item -LiteralPath (Join-Path $sourceRoot 'src') -Destination $fixture -Recurse -Force
$package = @'
{
  "name": "web-render-himawari-test", "private": true, "type": "module",
  "dependencies": { "web-render": "file:../../packages/vi5", "@tonejs/midi": "2.0.28", "midi-file": "1.2.4", "p5": "2.3.2" }
}
'@
$config = @'
import fs from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { defineConfig } from "web-render/config";
export default defineConfig({
  name: "himawari-test",
  hookConsoleLog: true,
  vitePlugins: [
    {name: "himawari-api", config: () => ({
      cacheDir: fileURLToPath(new URL("./.vite-cache-complete", import.meta.url)),
      optimizeDeps: {include: ["@tonejs/midi", "midi-file", "p5", "web-render > @datastructures-js/priority-queue"]},
      resolve: {alias: {
      vi5: fileURLToPath(new URL("../../packages/vi5/src/index.ts", import.meta.url)),
      p5: fileURLToPath(new URL("./node_modules/p5/dist/app.js", import.meta.url)),
    }}})},
    {name: "tonejs-mid", async transform(_, id) {
      if (!id.endsWith("?mid")) return;
      const file = id.replace(/\?mid$/, "");
      this.addWatchFile(file);
      const buffer = await fs.readFile(file);
      return `import { Midi } from "@tonejs/midi";
        import { parseMidi } from "midi-file";
        const buffer = new Uint8Array([${buffer.join(",")}]);
        export const toneJsMidi = new Midi(buffer);
        export const rawMidi = parseMidi(buffer);
        export default toneJsMidi;`;
    }},
  ],
});
'@
[IO.File]::WriteAllText((Join-Path $fixture 'package.json'),$package,[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $fixture 'web-render.config.ts'),$config,[Text.UTF8Encoding]::new($false))
& cargo run --locked -p web-render-cef --example generate_himawari_midi -- (Join-Path $proof 'generated-himawari.mid')
if ($LASTEXITCODE) { throw 'MIDI generation failed' }
# Preserve imports and source files; substitute only the MIDI in the test copy.
Copy-Item -LiteralPath (Join-Path $proof 'generated-himawari.mid') -Destination (Join-Path $fixture 'src/20260608_himawari_receiver.mid') -Force
& npm install --prefix $fixture --ignore-scripts
if ($LASTEXITCODE) { throw 'Test dependency installation failed' }
& node (Join-Path $root 'scripts/validate-himawari-midi.mjs') $fixture $proof
if ($LASTEXITCODE) { throw 'Generated MIDI validation failed' }
Write-Output "Prepared fixture: $fixture"
