import { defineHtmlObject } from "web-render";
export default defineHtmlObject({
  id: "frame-proof", label: "フレーム検証", source: "/pages/frame-proof.html",
  width: 320, height: 180,
  parameters: { caption: { type: "text", label: "文字", default: "Frame proof" } },
});
