import { defineHtmlObject } from "web-render";
export default defineHtmlObject({
  id: "html-title",
  label: "HTMLタイトル",
  source: "/pages/title.html",
  parameters: {
    title: { type: "text", label: "タイトル", default: "WEB → AVIUTL2" },
    accent: { type: "color", label: "アクセント", default: { r: 255, g: 70, b: 130, a: 255 } },
  },
});
