import type { ParameterDefinitions } from "./object";

/** A local project page. CSS/SVG and an optional window.aviutl.render hook are frame driven. */
export interface HtmlObject<T extends ParameterDefinitions = ParameterDefinitions> {
  kind: "html";
  id: string;
  label: string;
  parameters: T;
  /** Same-origin project URL, e.g. /pages/title.html */
  source: string;
  /** Omit to use the AviUtl2 project size. */
  width?: number;
  height?: number;
  timeoutMs?: number;
}

export function defineHtmlObject<T extends ParameterDefinitions>(
  options: Omit<HtmlObject<T>, "kind">,
): HtmlObject<T> {
  return { ...options, kind: "html" };
}
