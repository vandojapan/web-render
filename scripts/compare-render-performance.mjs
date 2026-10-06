import { readFile, readdir, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { resolve } from "node:path";
import assert from "node:assert/strict";

const proof = resolve(process.argv[2] ?? "docs/proofs/performance");
const readJson = async (path) => JSON.parse((await readFile(path, "utf8")).replace(/^\uFEFF/, ""));
const hash = async (path) => createHash("sha256").update(await readFile(path)).digest("hex");
const comparisons = [];
const timing = [];
for (const profile of ["debug", "release"]) {
  const beforePath = resolve(proof, `before-${profile}`);
  const afterPath = resolve(proof, `after-${profile}-isolated`);
  const before = await readJson(resolve(beforePath, "result.json"));
  const after = await readJson(resolve(afterPath, "result.json"));
  assert.equal(before.status, "passed");
  assert.equal(after.status, "passed");
  assert.equal(after.image_requests, 95);
  assert.equal(after.graceful_shutdown, true);
  for (const object of before.objects) {
    const updated = after.objects.find(o => o.object === object.object);
    assert.ok(updated);
    assert.equal(updated.reverse_seek_full_rgba, true);
    assert.ok(Object.values(updated.checks).every(Boolean));
    timing.push({profile, object: object.object, before_median_ms: object.timing.median_ms,
      after_median_ms: updated.timing.median_ms, before_p95_ms: object.timing.p95_ms,
      after_p95_ms: updated.timing.p95_ms, steady_samples: updated.timing.steady_samples,
      speedup: object.timing.median_ms / updated.timing.median_ms,
      reduction_percent: 100 * (1 - updated.timing.median_ms / object.timing.median_ms),
    });
  }
  for (const file of (await readdir(beforePath)).filter(f => f.endsWith(".png"))) {
    const original = await hash(resolve(beforePath, file));
    const updated = await hash(resolve(afterPath, file));
    assert.equal(original, updated, `${profile} ${file}: pixels changed`);
    comparisons.push({profile, file, sha256: original, identical: true});
  }
}
const host = await readJson(resolve(proof, "aviutl-after/result.json"));
assert.equal(host.status, "passed");
assert.equal(host.reverse_seek_full_rgba, true);
const hostTimes = host.frames.map(f => f.render_ms).sort((a,b) => a-b);
const hostComparisons = [];
const originalHost = resolve(proof, "../himawari/aviutl-replay");
for (const file of (await readdir(originalHost)).filter(f => f.endsWith(".png"))) {
  const original = await hash(resolve(originalHost, file));
  const updated = await hash(resolve(proof, "aviutl-after", file));
  assert.equal(original, updated, `AviUtl2 ${file}: pixels changed`);
  hostComparisons.push({file, sha256: original, identical: true});
}
const htmlComparisons = [];
const htmlOriginal = resolve(proof, "../debug");
for (const file of (await readdir(htmlOriginal)).filter(f => /^cef-frame-\d+\.png$/.test(f))) {
  const original = await hash(resolve(htmlOriginal, file));
  const updated = await hash(resolve(proof, "html-debug", file));
  assert.equal(original, updated, `HTML ${file}: pixels changed`);
  htmlComparisons.push({file, sha256: original, identical: true});
}
const result = {
  date: "2026-10-06", status: "passed", timing,
  protocol_image_comparisons: comparisons, host_image_comparisons: hostComparisons,
  html_image_comparisons: htmlComparisons,
  aviutl_scene: {objects: 3, captures: host.frames.length,
    median_ms: hostTimes[hostTimes.length / 2], p95_ms: hostTimes[Math.ceil(hostTimes.length * 0.95) - 1],
    includes: "SDK rendering and copying 1920x1080 RGBA; excludes warmup, PNG encoding, pixel validation",
  },
  cef_measurement: "IPC + actual CEF CPU paint; first setup and PNG encoding excluded; no frame-cache shortcut",
  concurrent_work: "Before-release ran during the new debug build. Initial after runs overlapped build/tests or HTML verification; isolated after runs are sequential without other test/build processes.",
  latency_limitations: "Release Synth Solo still has occasional IPC round-trip outliers (isolated p95 920.6 ms). Diagnostic render-loop batches complete in about 15–42 ms while some round trips exceed 1 s. Exact cause outside that phase is unresolved; image-validation passed does not mean every latency percentile improved.",
};
await writeFile(resolve(proof, "comparison.json"), JSON.stringify(result, null, 2));
console.log(JSON.stringify({status: result.status, timing, aviutl_scene: result.aviutl_scene,
  identical_cef_pngs: comparisons.length, identical_host_pngs: hostComparisons.length, identical_html_pngs: htmlComparisons.length}, null, 2));
