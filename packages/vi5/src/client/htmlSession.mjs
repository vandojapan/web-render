/** Chromium paints these iframes directly; no DOM-to-canvas emulation is used. */
export const METADATA_ROWS = 64;
export const SURFACE_WIDTH = 4096;
export const SURFACE_HEIGHT = 2304;

function bounded(promise, ms, label) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`${label}: timeout (${ms} ms)`)), ms);
    }),
  ]).finally(() => clearTimeout(timer));
}

export function resolvePageURL(source, base = location.href) {
  const url = new URL(source, base);
  const origin = new URL(base);
  if (url.origin !== origin.origin || !["http:", "https:"].includes(url.protocol)) {
    throw new Error("HTML pages must be served from the project origin");
  }
  if (url.pathname === "/vi5") throw new Error("Cannot render the plugin runtime recursively");
  return url.href;
}

export function validateSize(width, height) {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 ||
      width > SURFACE_WIDTH || height > SURFACE_HEIGHT - METADATA_ROWS) {
    throw new Error(`Invalid HTML viewport: ${width}x${height}; maximum 4096x2240`);
  }
}

export async function syncAnimations(doc, seconds) {
  for (const animation of doc.getAnimations()) {
    animation.pause();
    animation.currentTime = seconds * 1000;
  }
  for (const svg of doc.querySelectorAll("svg")) {
    if (typeof svg.pauseAnimations === "function") {
      svg.pauseAnimations();
      svg.setCurrentTime(seconds);
    }
  }
}

export class HtmlSession {
  constructor(source, width, height, timeoutMs = 15000) {
    validateSize(width, height);
    this.width = width;
    this.height = height;
    this.timeoutMs = timeoutMs;
    this.disposed = false;
    this.source = resolvePageURL(source);
    this.iframe = document.createElement("iframe");
    this.iframe.setAttribute("sandbox", "allow-scripts allow-same-origin");
    this.iframe.setAttribute("aria-hidden", "true");
    Object.assign(this.iframe.style, {
      position: "fixed", left: "0px", top: `${METADATA_ROWS}px`,
      width: `${width}px`, height: `${height}px`, border: "0", margin: "0",
      visibility: "hidden", background: "transparent", pointerEvents: "none", zIndex: "1",
    });
    this.ready = this.load();
  }

  async load() {
    try {
      const loaded = new Promise((resolve, reject) => {
        this.iframe.onload = resolve;
        this.iframe.onerror = () => reject(new Error(`Failed to load ${this.source}`));
      });
      this.iframe.src = this.source;
      document.body.append(this.iframe);
      await bounded(loaded, this.timeoutMs, "HTML load");
      this.assertActive();
      const doc = this.iframe.contentDocument;
      if (!doc || this.iframe.contentWindow.location.origin !== location.origin) {
        throw new Error("HTML page redirected outside the project origin");
      }
      await bounded(doc.fonts.ready, this.timeoutMs, "Fonts");
      await this.waitImages(doc);
      return doc;
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  async waitImages(doc) {
    await bounded(Promise.all([...doc.images].map(async image => {
      if (image.currentSrc || image.src) await image.decode();
    })), this.timeoutMs, "Images");
  }

  async render(frame, params) {
    try {
    const doc = await this.ready;
    this.assertActive();
    if (!Number.isFinite(frame.currentTime) || frame.currentTime < 0) {
      throw new Error("Frame time must be finite and non-negative");
    }
    const win = this.iframe.contentWindow;
    // HTML media require a separate seek/decode handshake and are deliberately excluded in v0.1.
    if (doc.querySelector("video,audio")) throw new Error("HTML video/audio are not supported in v0.1");
    const state = Object.freeze({ ...frame, width: this.width, height: this.height });
    win.aviutl ??= {};
    win.aviutl.frame = state;
    win.aviutl.params = params;
    await syncAnimations(doc, frame.currentTime);
    const hook = win.aviutl.render;
    if (typeof hook === "function") {
      await bounded(Promise.resolve(hook(state, params)), this.timeoutMs, "aviutl.render");
      this.assertActive();
    }
    await bounded(doc.fonts.ready, this.timeoutMs, "Fonts after render");
    await this.waitImages(doc);
    this.assertActive();
    await syncAnimations(doc, frame.currentTime);
    this.iframe.style.visibility = "visible";
    // Layout is flushed before the host writes a valid capture header.
    doc.documentElement.getBoundingClientRect();
    await bounded(new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))),
      this.timeoutMs, "Compositor settling");
    this.assertActive();
    if (doc.querySelector("video,audio")) throw new Error("HTML video/audio are not supported in v0.1");
    } catch(error) {
      // A timed-out hook cannot paint into the document used for the next request.
      this.dispose();
      throw error;
    }
  }

  assertActive() {
    if(this.disposed || !this.iframe.isConnected) throw new Error("HTML session was disposed");
  }
  hide() { this.iframe.style.visibility = "hidden"; }
  dispose() { this.disposed = true; this.iframe.remove(); }
}
