// Adapt the imported MIDI visualizers without changing the upstream source files
// or the renderer's scene-relative globalTime/globalFrame contract.
const clockConsumers = /\/src\/(?:utils\/currentFrameInfo\.ts|objects\/(?:drum|kaiwaiPhrase)\.object\.ts)$/;

export function himawariObjectClock({ timeBase = "object" } = {}) {
  if (!["object", "scene"].includes(timeBase)) throw new Error("Unknown MIDI time base");
  return {
    name: "himawari-object-clock",
    enforce: "pre",
    transform(source, id) {
      const path = id.replaceAll("\\", "/").split("?")[0];
      if (timeBase === "scene" || !clockConsumers.test(path)) return;
      const code = source.replaceAll("ctx.frameInfo.globalTime", "ctx.frameInfo.currentTime");
      if (code === source) throw new Error(`Himawari MIDI clock consumer changed: ${path}`);
      return { code, map: null };
    },
  };
}
