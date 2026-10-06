import { readFile, readdir, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { resolve } from "node:path";
import assert from "node:assert/strict";

const root = resolve(process.argv[2] ?? "docs/proofs/latency");
const readText = async path => {
  const bytes = await readFile(path);
  return bytes.toString(bytes[0] === 0xff && bytes[1] === 0xfe ? "utf16le" : "utf8").replace(/^\uFEFF/, "");
};
const readJson = async path => JSON.parse(await readText(path));
const hash = async path => createHash("sha256").update(await readFile(path)).digest("hex");
const stats = values => {
  const sorted = values.toSorted((a, b) => a - b);
  assert.ok(sorted.length > 0);
  return { samples: sorted.length, median_ms: sorted[Math.floor(sorted.length / 2)],
    p95_ms: sorted[Math.ceil(sorted.length * .95) - 1],
    p99_ms: sorted[Math.ceil(sorted.length * .99) - 1], max_ms: sorted.at(-1),
    over_250_ms: sorted.filter(x => x > 250).length };
};
const results = [];
const baselinePath = resolve(root, "before-stress");
const baselineFiles = (await readdir(baselinePath)).filter(f => f.endsWith(".png"));
for (const run of ["before-stress", "after-release", "after-release-quiet", "after-compression", "final-release", "final-release-repeat", "final-debug"]) {
  const path = resolve(root, run);
  const result = await readJson(resolve(path, "result.json"));
  assert.equal(result.status, "passed");
  assert.equal(result.graceful_shutdown, true);
  assert.equal(result.stress.length, 600);
  assert.equal(result.image_requests, 695);
  for (const object of result.objects) {
    assert.equal(object.reverse_seek_full_rgba, true);
    assert.ok(Object.values(object.checks).every(Boolean));
  }
  const images = [];
  for (const file of baselineFiles) {
    const sha256 = await hash(resolve(path, file));
    assert.equal(sha256, await hash(resolve(baselinePath, file)), `${run}/${file}`);
    images.push({ file, sha256 });
  }
  const log = await readText(resolve(root, `${run}.log`));
  const parentMs = [...log.matchAll(/Parent process check finished elapsed_ms=([\d.]+)/g)].map(m => Number(m[1]));
  results.push({ run, stress: stats(result.stress.map(f => f.render_ms)),
    objects: result.objects.map(o => ({ object: o.object, ...stats(o.frames.slice(1).map(f => f.render_ms)) })),
    parent_checks: parentMs.length ? stats(parentMs) : null,
    identical_pngs: images.length, images,
  });
}
const host = await readJson(resolve(root, "aviutl-final/result.json"));
assert.equal(host.status, "passed");
assert.equal(host.reverse_seek_full_rgba, true);
const runtimeImages = [];
for (const [label, original, updated] of [
  ["aviutl", "../performance/aviutl-after", "aviutl-final"],
  ["html", "../performance/html-debug", "html-debug"],
]) {
  const files = (await readdir(resolve(root, original))).filter(f => f.endsWith(".png"));
  for (const file of files) {
    const sha256 = await hash(resolve(root, updated, file));
    assert.equal(sha256, await hash(resolve(root, original, file)), `${label}/${file}`);
    runtimeImages.push({ label, file, sha256 });
  }
}
const output = { date: "2026-10-06", status: "passed", runs: results,
  aviutl_scene: stats(host.frames.map(f => f.render_ms)), runtime_images: runtimeImages,
  measurement: "Sequential actual CEF IPC, fixed 600 stress requests with full RGBA equality checked on each request; PNG encoding and comparison excluded from timing; same MIDI and frames; upper middle median.",
};
await writeFile(resolve(root, "comparison.json"), JSON.stringify(output, null, 2));
console.log(JSON.stringify({ runs: results.map(({ images, ...r }) => r), aviutl_scene: output.aviutl_scene, identical_runtime_pngs: runtimeImages.length }, null, 2));
