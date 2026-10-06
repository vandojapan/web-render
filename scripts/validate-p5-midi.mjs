import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const [project, output] = process.argv.slice(2);
if (!project || !output) throw new Error("validate-p5-midi <prepared-p5-project> <report-directory>");
const { parseSmf, secondsToTick, tickToEighthPosition } = await import(
  pathToFileURL(path.resolve(project, "src/midi/smf.ts")).href,
);
const bytes = await fs.readFile(path.join(project, "public/song.mid"));
const midi = parseSmf(bytes);
assert.equal(midi.format, 0);
assert.equal(midi.ppq, 480);
assert.equal(midi.tracks.length, 1);
assert.equal(midi.tracks[0].length, 24);
assert.equal(midi.durationTicks, 5760);
assert.deepEqual(midi.tempoMap.map(s => s.microsecondsPerQuarter), [500000, 400000]);
for (const [index, note] of midi.tracks[0].entries()) {
  assert.equal(note.note, 60 + index % 12);
  assert.equal(note.startTick, index * 240);
  assert.equal(note.endTick, (index + 1) * 240);
}
const eighth = tickToEighthPosition(midi, secondsToTick(midi, 2.8));
const report = {
  format: midi.format, ppq: midi.ppq, bytes: bytes.length,
  tracks: midi.tracks.length, notes: midi.tracks[0].length,
  tempo_bpm: midi.tempoMap.map(s => 60000000 / s.microsecondsPerQuarter),
  duration_ticks: midi.durationTicks, duration_seconds: 5.2,
  boundary: { seconds: 2.8, expected_eighth: 12, actual_eighth: eighth,
    actual_step: Math.floor(eighth), issue: Math.floor(eighth) !== 12 },
};
await fs.mkdir(output, { recursive: true });
await fs.writeFile(path.join(output, "midi-summary.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
