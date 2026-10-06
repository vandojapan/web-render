import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";

const fixture = resolve(process.argv[2] ?? ".local-tests/himawari-receiver");
const proof = resolve(process.argv[3] ?? "docs/proofs/himawari");
const require = createRequire(pathToFileURL(resolve(fixture, "package.json")));
const { Midi } = require("@tonejs/midi");
const { parseMidi } = require("midi-file");
const buffer = await readFile(resolve(proof, "generated-himawari.mid"));
const raw = parseMidi(buffer);
const midi = new Midi(buffer);
assert.equal(raw.header.format, 1);
assert.equal(raw.header.ticksPerBeat, 480);
assert.equal(raw.tracks.length, 15);
const names = ["drums", "sub-cymbal-a", "sub-cymbal-b", "drums-unused", "cymbal-a", "cymbal-b", "main-drums-a", "main-drums-b", "synth", "synth-unused", "synth-a", "synth-b", "Kaiwai Phrase", "phrase-notes"];
assert.deepEqual(midi.tracks.map(t => t.name), names);
assert.deepEqual(midi.header.tempos.map(t => [t.ticks, t.bpm]), [[0, 120], [7680, 150]]);
assert.deepEqual(midi.header.timeSignatures[0].timeSignature, [4, 4]);
assert.equal(midi.durationTicks, 15360);
assert.equal(midi.duration, 14.4);
assert.equal(midi.header.secondsToTicks(8), 7680);
assert.equal(midi.header.ticksToMeasures(7680), 4);
assert.equal(midi.tracks.reduce((n, t) => n + t.notes.length, 0), 184);
for (const [index, count] of [[1, 8], [2, 8], [4, 8], [5, 8], [6, 32], [7, 64], [10, 16], [11, 16], [13, 24]]) assert.equal(midi.tracks[index].notes.length, count);
assert.deepEqual([...new Set(midi.tracks.slice(6, 8).flatMap(t => t.notes.map(n => n.midi)))].sort((a,b) => a-b), [36, 37, 38, 39, 41]);
for (const index of [10, 11]) {
  assert.equal(midi.tracks[index].pitchBends.length, 64);
  assert.deepEqual(midi.tracks[index].pitchBends.slice(0, 4).map(b => [b.ticks, b.value]), [[0, 0], [240, 0.5], [480, -0.5], [720, 0]]);
}
for (const track of midi.tracks) for (const note of track.notes) assert.ok(note.durationTicks > 0 && note.ticks + note.durationTicks <= 15360);
const hashes = JSON.parse((await readFile(resolve(proof, "source-hashes.json"), "utf8")).replace(/^\uFEFF/, ""));
for (const entry of hashes) assert.equal(createHash("sha256").update(await readFile(entry.path)).digest("hex").toUpperCase(), entry.sha256);
for (const entry of hashes.filter(e => e.path.endsWith(".ts"))) {
  const relative = entry.path.split(/[\\/]src[\\/]/)[1];
  assert.equal(createHash("sha256").update(await readFile(resolve(fixture, "src", relative))).digest("hex").toUpperCase(), entry.sha256);
}
const result = {
  status: "passed", format: 1, ppq: 480, raw_tracks: 15, tonejs_tracks: 14,
  notes: 184, bars: 8, duration_seconds: midi.duration, duration_ticks: midi.durationTicks,
  tempo_change: {tick: 7680, time: 8, from_bpm: 120, to_bpm: 150},
  tracks: midi.tracks.map((t, index) => ({index, name: t.name, notes: t.notes.length, pitch_bends: t.pitchBends.length})),
  original_source_and_midi_unchanged: true, object_and_util_typescript_unchanged: true,
  sha256: createHash("sha256").update(buffer).digest("hex"),
};
await writeFile(resolve(proof, "midi-summary.json"), JSON.stringify(result, null, 2));
console.log(JSON.stringify(result, null, 2));
