const test = require("node:test");
const assert = require("node:assert/strict");

test("MIDI time follows the object start in every himawari clock consumer", async () => {
  const { himawariObjectClock } = await import("../scripts/himawari-object-clock.mjs");
  const plugin = himawariObjectClock();
  for (const path of ["utils/currentFrameInfo.ts", "objects/drum.object.ts", "objects/kaiwaiPhrase.object.ts"]) {
    const source = "const t = ctx.frameInfo.globalTime; const initial = ctx.frameInfo.globalTime - ctx.frameInfo.currentTime;";
    const result = plugin.transform(source, `E:\\fixture\\src\\${path.replaceAll("/", "\\")}?v=1`);
    const evaluate = new Function("ctx", `${result.code}; return [t, initial];`);
    assert.deepEqual(evaluate({ frameInfo: { globalTime: 14, currentTime: 2 } }), [2, 0]);
    assert.deepEqual(evaluate({ frameInfo: { globalTime: 22, currentTime: 2 } }), [2, 0]);
    assert.ok(source.includes("globalTime"));
  }
});

test("Scene synchronization and unrelated sources retain their clock", async () => {
  const { himawariObjectClock } = await import("../scripts/himawari-object-clock.mjs");
  const source = "ctx.frameInfo.globalTime";
  assert.equal(himawariObjectClock({ timeBase: "scene" }).transform(source, "/fixture/src/utils/currentFrameInfo.ts"), undefined);
  assert.equal(himawariObjectClock().transform(source, "/other/src/objects/other.object.ts"), undefined);
});
