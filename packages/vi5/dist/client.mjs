import { a as ParameterDefinitionSchema, c as file_common, i as ObjectInfoSchema, n as BatchRenderRequestSchema, o as ParameterSchema, s as ParameterTypeSchema, t as Vi5Context } from "./context-BmtDCMcB.mjs";
import { fileDesc, messageDesc } from "@bufbuild/protobuf/codegenv2";
import p5 from "p5";
import * as logtape from "@logtape/logtape";
import * as fastBase64 from "fast-base64";
import * as protobuf from "@bufbuild/protobuf";
import { PriorityQueue } from "@datastructures-js/priority-queue";

//#region src/client/log.ts
const vi5Log = logtape.getLogger("vi5");
await logtape.configure({
	sinks: { console: logtape.getConsoleSink({ formatter: logtape.getTextFormatter({
		level: "full",
		category: (category) => `[${category.join("][")}]`
	}) }) },
	loggers: [{
		category: [],
		lowestLevel: "debug",
		sinks: ["console"]
	}, {
		category: ["logtape", "meta"],
		lowestLevel: "warning",
		sinks: ["console"]
	}]
});

//#endregion
//#region src/gen/server-js_pb.ts
/**
* Describes the file server-js.proto.
*/
const file_server_js = /* @__PURE__ */ fileDesc("Cg9zZXJ2ZXItanMucHJvdG8SCHNlcnZlcmpzIkAKDkluaXRpYWxpemVJbmZvEhQKDHByb2plY3RfbmFtZRgBIAEoCRIYChByZW5kZXJlcl92ZXJzaW9uGAIgASgJIksKFFJlbmRlcmVyZWRPYmplY3RJbmZvEgkKAXgYASABKAUSCQoBeRgCIAEoBRINCgV3aWR0aBgDIAEoBRIOCgZoZWlnaHQYBCABKAUijAEKFFNpbmdsZVJlbmRlclJlc3BvbnNlEg0KBW5vbmNlGAEgASgFEkAKFnJlbmRlcmVyZWRfb2JqZWN0X2luZm8YAiABKAsyHi5zZXJ2ZXJqcy5SZW5kZXJlcmVkT2JqZWN0SW5mb0gAEhcKDWVycm9yX21lc3NhZ2UYAyABKAlIAEIKCghyZXNwb25zZSJ1ChJSb290UmVuZGVyUmVzcG9uc2USOgoHc3VjY2VzcxgBIAEoCzInLnNlcnZlcmpzLk1heWJlSW5jb21wbGV0ZVJlbmRlclJlc3BvbnNlSAASFwoNZXJyb3JfbWVzc2FnZRgCIAEoCUgAQgoKCHJlc3BvbnNlIjkKA0xvZxIhCgVsZXZlbBgBIAEoDjISLnNlcnZlcmpzLkxvZ0xldmVsEg8KB21lc3NhZ2UYAiABKAkiSAocT2JqZWN0TGlzdFVwZGF0ZU5vdGlmaWNhdGlvbhIoCgxvYmplY3RfaW5mb3MYASADKAsyEi5jb21tb24uT2JqZWN0SW5mbyKAAQoRTm90aWZpY2F0aW9uRW50cnkSHAoDbG9nGAEgASgLMg0uc2VydmVyanMuTG9nSAASRAoSb2JqZWN0X2xpc3RfdXBkYXRlGAIgASgLMiYuc2VydmVyanMuT2JqZWN0TGlzdFVwZGF0ZU5vdGlmaWNhdGlvbkgAQgcKBWVudHJ5Ij0KDU5vdGlmaWNhdGlvbnMSLAoHZW50cmllcxgBIAMoCzIbLnNlcnZlcmpzLk5vdGlmaWNhdGlvbkVudHJ5InAKHU1heWJlSW5jb21wbGV0ZVJlbmRlclJlc3BvbnNlEjgKEHJlbmRlcl9yZXNwb25zZXMYASADKAsyHi5zZXJ2ZXJqcy5TaW5nbGVSZW5kZXJSZXNwb25zZRIVCg1pc19pbmNvbXBsZXRlGAIgASgIKkcKCExvZ0xldmVsEhIKDkxPR19MRVZFTF9JTkZPEAASEgoOTE9HX0xFVkVMX1dBUk4QARITCg9MT0dfTEVWRUxfRVJST1IQAmIGcHJvdG8z", [file_common]);
/**
* Describes the message serverjs.InitializeInfo.
* Use `create(InitializeInfoSchema)` to create a new message.
*/
const InitializeInfoSchema = /* @__PURE__ */ messageDesc(file_server_js, 0);
/**
* Describes the message serverjs.RendereredObjectInfo.
* Use `create(RendereredObjectInfoSchema)` to create a new message.
*/
const RendereredObjectInfoSchema = /* @__PURE__ */ messageDesc(file_server_js, 1);
/**
* Describes the message serverjs.SingleRenderResponse.
* Use `create(SingleRenderResponseSchema)` to create a new message.
*/
const SingleRenderResponseSchema = /* @__PURE__ */ messageDesc(file_server_js, 2);
/**
* Describes the message serverjs.RootRenderResponse.
* Use `create(RootRenderResponseSchema)` to create a new message.
*/
const RootRenderResponseSchema = /* @__PURE__ */ messageDesc(file_server_js, 3);
/**
* Describes the message serverjs.Notifications.
* Use `create(NotificationsSchema)` to create a new message.
*/
const NotificationsSchema = /* @__PURE__ */ messageDesc(file_server_js, 7);
/**
* Describes the message serverjs.MaybeIncompleteRenderResponse.
* Use `create(MaybeIncompleteRenderResponseSchema)` to create a new message.
*/
const MaybeIncompleteRenderResponseSchema = /* @__PURE__ */ messageDesc(file_server_js, 8);
/**
* @generated from enum serverjs.LogLevel
*/
let LogLevel = /* @__PURE__ */ function(LogLevel) {
	/**
	* @generated from enum value: LOG_LEVEL_INFO = 0;
	*/
	LogLevel[LogLevel["INFO"] = 0] = "INFO";
	/**
	* @generated from enum value: LOG_LEVEL_WARN = 1;
	*/
	LogLevel[LogLevel["WARN"] = 1] = "WARN";
	/**
	* @generated from enum value: LOG_LEVEL_ERROR = 2;
	*/
	LogLevel[LogLevel["ERROR"] = 2] = "ERROR";
	return LogLevel;
}({});

//#endregion
//#region src/client/htmlSession.mjs
/** Chromium paints these iframes directly; no DOM-to-canvas emulation is used. */
const METADATA_ROWS = 64;
const SURFACE_WIDTH = 4096;
const SURFACE_HEIGHT = 2304;
function bounded(promise, ms, label) {
	let timer;
	return Promise.race([promise, new Promise((_, reject) => {
		timer = setTimeout(() => reject(/* @__PURE__ */ new Error(`${label}: timeout (${ms} ms)`)), ms);
	})]).finally(() => clearTimeout(timer));
}
function resolvePageURL(source, base = location.href) {
	const url = new URL(source, base);
	const origin = new URL(base);
	if (url.origin !== origin.origin || !["http:", "https:"].includes(url.protocol)) throw new Error("HTML pages must be served from the project origin");
	if (url.pathname === "/vi5") throw new Error("Cannot render the plugin runtime recursively");
	return url.href;
}
function validateSize(width, height) {
	if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 || width > SURFACE_WIDTH || height > SURFACE_HEIGHT - METADATA_ROWS) throw new Error(`Invalid HTML viewport: ${width}x${height}; maximum 4096x2240`);
}
async function syncAnimations(doc, seconds) {
	for (const animation of doc.getAnimations()) {
		animation.pause();
		animation.currentTime = seconds * 1e3;
	}
	for (const svg of doc.querySelectorAll("svg")) if (typeof svg.pauseAnimations === "function") {
		svg.pauseAnimations();
		svg.setCurrentTime(seconds);
	}
}
var HtmlSession = class {
	constructor(source, width, height, timeoutMs = 15e3) {
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
			position: "fixed",
			left: "0px",
			top: `${METADATA_ROWS}px`,
			width: `${width}px`,
			height: `${height}px`,
			border: "0",
			margin: "0",
			visibility: "hidden",
			background: "transparent",
			pointerEvents: "none",
			zIndex: "1"
		});
		this.ready = this.load();
	}
	async load() {
		try {
			const loaded = new Promise((resolve, reject) => {
				this.iframe.onload = resolve;
				this.iframe.onerror = () => reject(/* @__PURE__ */ new Error(`Failed to load ${this.source}`));
			});
			this.iframe.src = this.source;
			document.body.append(this.iframe);
			await bounded(loaded, this.timeoutMs, "HTML load");
			this.assertActive();
			const doc = this.iframe.contentDocument;
			if (!doc || this.iframe.contentWindow.location.origin !== location.origin) throw new Error("HTML page redirected outside the project origin");
			await bounded(doc.fonts.ready, this.timeoutMs, "Fonts");
			await this.waitImages(doc);
			return doc;
		} catch (error) {
			this.dispose();
			throw error;
		}
	}
	async waitImages(doc) {
		await bounded(Promise.all([...doc.images].map(async (image) => {
			if (image.currentSrc || image.src) await image.decode();
		})), this.timeoutMs, "Images");
	}
	async render(frame, params) {
		try {
			const doc = await this.ready;
			this.assertActive();
			if (!Number.isFinite(frame.currentTime) || frame.currentTime < 0) throw new Error("Frame time must be finite and non-negative");
			const win = this.iframe.contentWindow;
			if (doc.querySelector("video,audio")) throw new Error("HTML video/audio are not supported in v0.1");
			const state = Object.freeze({
				...frame,
				width: this.width,
				height: this.height
			});
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
			doc.documentElement.getBoundingClientRect();
			await bounded(new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))), this.timeoutMs, "Compositor settling");
			this.assertActive();
			if (doc.querySelector("video,audio")) throw new Error("HTML video/audio are not supported in v0.1");
		} catch (error) {
			this.dispose();
			throw error;
		}
	}
	assertActive() {
		if (this.disposed || !this.iframe.isConnected) throw new Error("HTML session was disposed");
	}
	hide() {
		this.iframe.style.visibility = "hidden";
	}
	dispose() {
		this.disposed = true;
		this.iframe.remove();
	}
};

//#endregion
//#region src/client/packCanvas.ts
const bytesPerPixel = 3;
const messageHeaderBytes = 11;
const buildErrorResponse = (nonce, error) => protobuf.create(SingleRenderResponseSchema, {
	nonce,
	response: {
		case: "errorMessage",
		value: error
	}
});
const buildSuccessResponse = (nonce, x, y, width, height) => protobuf.create(SingleRenderResponseSchema, {
	nonce,
	response: {
		case: "rendereredObjectInfo",
		value: protobuf.create(RendereredObjectInfoSchema, {
			x,
			y,
			width,
			height
		})
	}
});
const getMetadataRows = (renderResponses) => {
	const payloadLength = protobuf.toBinary(MaybeIncompleteRenderResponseSchema, protobuf.create(MaybeIncompleteRenderResponseSchema, {
		renderResponses,
		isIncomplete: true
	})).length + messageHeaderBytes;
	const pixels = Math.ceil(payloadLength / bytesPerPixel);
	if (Math.ceil(pixels / Vi5Runtime.get().canvas.width) > METADATA_ROWS) throw new Error("Packed metadata exceeds reserved rows");
	return METADATA_ROWS;
};
const packBatch = (responses, startIndex, metadataRows) => {
	const renderResponses = [];
	const packedCanvases = [];
	let x = 0;
	let y = metadataRows;
	let rowHeight = 0;
	let index = startIndex;
	while (index < responses.length) {
		const response = responses[index];
		if (response.type === "error") {
			renderResponses.push(buildErrorResponse(response.renderNonce, response.error));
			index += 1;
			continue;
		}
		const width = response.width;
		const height = response.height;
		if (width > Vi5Runtime.get().canvas.width || height > Vi5Runtime.get().canvas.height - metadataRows) {
			renderResponses.push(buildErrorResponse(response.renderNonce, "canvas size exceeds pack area"));
			index += 1;
			continue;
		}
		if (x + width > Vi5Runtime.get().canvas.width) {
			x = 0;
			y += rowHeight;
			rowHeight = 0;
		}
		if (y + height > Vi5Runtime.get().canvas.height) {
			if (renderResponses.length === 0) renderResponses.push(buildErrorResponse(response.renderNonce, "canvas size exceeds pack area"));
			break;
		}
		renderResponses.push(buildSuccessResponse(response.renderNonce, x, y, width, height));
		packedCanvases.push({
			canvas: response.canvas,
			x,
			y
		});
		x += width;
		index += 1;
		rowHeight = Math.max(rowHeight, height);
	}
	const updatedMetadataRows = getMetadataRows(renderResponses);
	return {
		nextIndex: index,
		renderResponses,
		packedCanvases,
		metadataRows: updatedMetadataRows
	};
};
function packCanvases(responses) {
	const batches = [];
	let startIndex = 0;
	while (startIndex < responses.length) {
		let metadataRows = METADATA_ROWS;
		let result;
		for (;;) {
			result = packBatch(responses, startIndex, metadataRows);
			if (result.metadataRows === metadataRows) break;
			metadataRows = result.metadataRows;
		}
		startIndex = result.nextIndex;
		batches.push(protobuf.create(MaybeIncompleteRenderResponseSchema, {
			renderResponses: result.renderResponses,
			isIncomplete: true
		}));
	}
	batches[batches.length - 1].isIncomplete = false;
	return batches;
}

//#endregion
//#region src/client/disposableCounter.ts
const log = vi5Log.getChild("disposableCounter");
var DisposableCounterFactory = class {
	#count = 0;
	createCounter() {
		this.#count++;
		log.debug`Created counter, total count: ${this.#count}`;
		return new DisposableCounter(() => this.disposeCounter());
	}
	get count() {
		return this.#count;
	}
	disposeCounter() {
		this.#count--;
		log.debug`Disposed counter, total count: ${this.#count}`;
	}
};
var DisposableCounter = class {
	disposed = false;
	constructor(onDispose) {
		this.onDispose = onDispose;
	}
	[Symbol.dispose]() {
		if (!this.disposed) {
			this.onDispose();
			this.disposed = true;
		}
	}
};

//#endregion
//#region src/client/renderQueue.ts
const priorityLevels = {
	init: 100,
	render: 10,
	notify: 1
};
var RenderQueue = class {
	#queue = new PriorityQueue((a, b) => {
		return b.priority - a.priority || a.insertedAt - b.insertedAt;
	});
	#raf = null;
	constructor() {
		const processQueue = () => {
			while (true) {
				const task = this.#queue.dequeue();
				if (task) {
					if (task.task() !== "skip") break;
				} else break;
			}
			this.#raf = requestAnimationFrame(processQueue);
		};
		this.#raf = requestAnimationFrame(processQueue);
	}
	render(priority, renderFunction) {
		const { promise, resolve, reject } = Promise.withResolvers();
		this.#queue.push({
			task: () => {
				try {
					const result = renderFunction();
					resolve();
					return result;
				} catch (error) {
					reject(error);
					return "skip";
				}
			},
			insertedAt: Date.now(),
			priority
		});
		return promise;
	}
};

//#endregion
//#region src/client/runtime.ts
const runtimeLog = vi5Log.getChild("Vi5Runtime");
const notificationNonce = 1;
const notificationLevelMap = {
	info: LogLevel.INFO,
	warn: LogLevel.WARN,
	error: LogLevel.ERROR
};
const isMessage = (data) => {
	return typeof data === "object" && data !== null && "$typeName" in data;
};
const initializePromises = /* @__PURE__ */ new Map();
const contexts = /* @__PURE__ */ new Map();
async function initializeContext(id, object, renderRequest, parameter) {
	if (contexts.has(id)) return contexts.get(id);
	if (initializePromises.has(id)) return initializePromises.get(id);
	const ctx = new Vi5Context();
	const { promise, resolve, reject } = Promise.withResolvers();
	initializePromises.set(id, promise);
	new p5((sketch) => {
		ctx.initialize(sketch);
		sketch.setup = async () => {
			sketch.noLoop();
			ctx.setFrameInfo(renderRequest.frameInfo);
			try {
				await object.setup(ctx, ctx.p, parameter);
				if (initializePromises.get(id) !== promise) {
					ctx.teardown();
					throw new Error("p5 initialization was invalidated");
				}
				contexts.set(id, ctx);
				resolve(ctx);
			} catch (error) {
				reject(error);
			}
		};
	});
	return promise;
}
function grpcParamsToJsParams(grpcParams) {
	const params = {};
	for (const param of grpcParams) switch (param.value.case) {
		case "strValue":
			params[param.key] = param.value.value;
			break;
		case "textValue":
			params[param.key] = param.value.value;
			break;
		case "numberValue":
			params[param.key] = param.value.value;
			break;
		case "boolValue":
			params[param.key] = param.value.value;
			break;
		case "colorValue":
			params[param.key] = {
				r: param.value.value.r,
				g: param.value.value.g,
				b: param.value.value.b,
				a: param.value.value.a
			};
			break;
		default: runtimeLog.warn`Unknown parameter value case: ${param.value.case}`;
	}
	return params;
}
function toGrpcParameterType(definition) {
	switch (definition.type) {
		case "string": return protobuf.create(ParameterTypeSchema, { kind: {
			case: "string",
			value: {}
		} });
		case "text": return protobuf.create(ParameterTypeSchema, { kind: {
			case: "text",
			value: {}
		} });
		case "boolean": return protobuf.create(ParameterTypeSchema, { kind: {
			case: "boolean",
			value: {}
		} });
		case "number": return protobuf.create(ParameterTypeSchema, { kind: {
			case: "number",
			value: {
				step: definition.step,
				min: definition.min,
				max: definition.max
			}
		} });
		case "color": return protobuf.create(ParameterTypeSchema, { kind: {
			case: "color",
			value: {}
		} });
	}
}
function toGrpcDefaultValue(key, definition) {
	if (definition.default === void 0) return;
	switch (definition.type) {
		case "string": return protobuf.create(ParameterSchema, {
			key,
			value: {
				case: "strValue",
				value: definition.default
			}
		});
		case "text": return protobuf.create(ParameterSchema, {
			key,
			value: {
				case: "textValue",
				value: definition.default
			}
		});
		case "number": return protobuf.create(ParameterSchema, {
			key,
			value: {
				case: "numberValue",
				value: definition.default
			}
		});
		case "boolean": return protobuf.create(ParameterSchema, {
			key,
			value: {
				case: "boolValue",
				value: definition.default
			}
		});
		case "color": return protobuf.create(ParameterSchema, {
			key,
			value: {
				case: "colorValue",
				value: {
					r: definition.default.r,
					g: definition.default.g,
					b: definition.default.b,
					a: definition.default.a
				}
			}
		});
	}
}
function toGrpcParameterDefinition(key, definition) {
	return protobuf.create(ParameterDefinitionSchema, {
		key,
		label: definition.label ?? key,
		type: toGrpcParameterType(definition),
		defaultValue: toGrpcDefaultValue(key, definition)
	});
}
const temporaryCanvases = [];
const cloneCanvas = (source) => {
	if (temporaryCanvases.length === 0) runtimeLog.debug`Adding new temporary canvas (pool size: ${temporaryCanvases.length + 1})`;
	const clone = temporaryCanvases.pop() ?? document.createElement("canvas");
	if (clone.width < source.width) {
		runtimeLog.debug`Resizing temporary canvas to width ${source.width}`;
		clone.width = source.width;
	}
	if (clone.height < source.height) {
		runtimeLog.debug`Resizing temporary canvas to height ${source.height}`;
		clone.height = source.height;
	}
	const ctx = clone.getContext("2d");
	ctx.clearRect(0, 0, source.width, source.height);
	ctx.drawImage(source, 0, 0, source.width, source.height);
	return clone;
};
const disposeCanvas = (canvas) => {
	temporaryCanvases.push(canvas);
};
var Vi5Runtime = class {
	canvas;
	ctx;
	objects = /* @__PURE__ */ new Map();
	#htmlSessions = /* @__PURE__ */ new Map();
	#captureNonce = 0;
	#generation = 0;
	#objectInfosDirty = false;
	#notificationsScheduled = false;
	#renderQueue = new RenderQueue();
	#notifyCounter = new DisposableCounterFactory();
	#logQueue = [];
	constructor(projectName) {
		this.projectName = projectName;
		this.canvas = document.getElementById("vi5-canvas");
		this.ctx = this.canvas.getContext("2d");
	}
	async init() {
		await this.#renderQueue.render(priorityLevels.init, () => {
			this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
			this.drawMessage(InitializeInfoSchema, {
				projectName: this.projectName,
				rendererVersion: "1.0.0"
			}, 0);
		});
		this.#notifyObjectInfos();
		if (import.meta.hot) import.meta.hot.on("vi5:on-object-list-changed", (_list) => {
			runtimeLog.info`Object list changed, reloading page...`;
			window.location.reload();
		});
	}
	#notifyObjectInfos() {
		this.#objectInfosDirty = true;
		this.#scheduleNotifications();
	}
	#scheduleNotifications() {
		if (this.#notificationsScheduled || this.#captureNonce !== null || !this.#objectInfosDirty && this.#logQueue.length === 0) return;
		this.#notificationsScheduled = true;
		this.#renderQueue.render(priorityLevels.notify, () => {
			this.#notificationsScheduled = false;
			if (this.#captureNonce !== null) return "skip";
			const counter = this.#notifyCounter.createCounter();
			try {
				const objectInfos = Array.from(this.objects.values()).map((obj) => protobuf.create(ObjectInfoSchema, {
					id: obj.id,
					label: obj.label,
					parameterDefinitions: Object.entries(obj.parameters).map(([key, def]) => toGrpcParameterDefinition(key, def))
				}));
				this.drawMessage(NotificationsSchema, { entries: [...this.#objectInfosDirty ? [{ entry: {
					case: "objectListUpdate",
					value: { objectInfos }
				} }] : [], ...this.#logQueue.map(({ level, message }) => ({ entry: {
					case: "log",
					value: {
						level: notificationLevelMap[level],
						message
					}
				} }))] }, notificationNonce);
				this.#objectInfosDirty = false;
				this.#logQueue.length = 0;
			} finally {
				counter[Symbol.dispose]();
			}
		});
	}
	/** Called by CEF only after it has copied the captured RGBA for this nonce. */
	acknowledge(nonce) {
		if (this.#captureNonce !== nonce) return;
		this.#captureNonce = null;
		this.#scheduleNotifications();
	}
	cancelCapture(nonce) {
		if (this.#captureNonce === nonce) this.purgeCache();
	}
	async render(nonce, dataB64) {
		if (this.#captureNonce !== null) throw new Error("Previous native capture has not been acknowledged");
		this.#captureNonce = nonce;
		const generation = this.#generation;
		try {
			const data = await fastBase64.toBytes(dataB64);
			const renderPayload = protobuf.fromBinary(BatchRenderRequestSchema, data);
			if (renderPayload.renderRequests.some((req) => "kind" in (this.objects.get(req.object) ?? {}))) {
				if (renderPayload.renderRequests.length !== 1) throw new Error("HTML requires single-frame capture");
				const req = renderPayload.renderRequests[0];
				await this.renderHtml(nonce, req, generation);
				return;
			}
			this.hideHtml();
			const jsResponses = [];
			for (const req of renderPayload.renderRequests) try {
				jsResponses.push(await this.doRender(req));
			} catch (e) {
				runtimeLog.error`Error during rendering object ${req.object}: ${String(e)}`;
				jsResponses.push({
					type: "error",
					renderNonce: req.renderNonce,
					error: `Error during rendering: ${String(e)}`
				});
			}
			const canvases = /* @__PURE__ */ new Map();
			for (const resp of jsResponses) if (resp.type === "success") canvases.set(resp.renderNonce, resp.canvas);
			const packed = packCanvases(jsResponses);
			for (const packedResponse of packed) await this.#renderQueue.render(priorityLevels.render, () => {
				if (generation === this.#generation) this.renderSingleResponse(packedResponse, nonce, canvases);
			});
			for (const canvas of canvases.values()) disposeCanvas(canvas);
			canvases.clear();
		} catch (e) {
			if (generation !== this.#generation) return;
			runtimeLog.error`Error during batch rendering: ${String(e)}`;
			this.drawMessage(RootRenderResponseSchema, { response: {
				case: "errorMessage",
				value: `Error during batch rendering: ${String(e)}`
			} }, nonce);
		}
	}
	renderSingleResponse(packedResponse, nonce, canvases) {
		this.drawMessage(RootRenderResponseSchema, { response: {
			case: "success",
			value: packedResponse
		} }, nonce);
		for (const renderResponse of packedResponse.renderResponses) if (renderResponse.response.case === "rendereredObjectInfo") {
			const info = renderResponse.response.value;
			this.ctx.clearRect(info.x, info.y, info.width, info.height);
			this.ctx.drawImage(canvases.get(renderResponse.nonce), 0, 0, info.width, info.height, info.x, info.y, info.width, info.height);
			runtimeLog.debug`Rendered object ${renderResponse.nonce} at (${info.x}, ${info.y}) with size ${info.width}x${info.height}`;
		}
	}
	hideHtml() {
		for (const session of this.#htmlSessions.values()) session.hide();
	}
	async renderHtml(nonce, request, generation) {
		this.hideHtml();
		this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
		try {
			const object = this.objects.get(request.object);
			if (!object || !("kind" in object) || object.kind !== "html") throw new Error("Not an HTML object");
			const frame = request.frameInfo;
			if (!frame) throw new Error("Missing frame info");
			const width = object.width ?? frame.screenWidth;
			const height = object.height ?? frame.screenHeight;
			const source = new URL(object.source, location.href).href;
			let session = this.#htmlSessions.get(request.objectId);
			if (session && (session.source !== source || session.width !== width || session.height !== height)) {
				session.dispose();
				this.#htmlSessions.delete(request.objectId);
				session = void 0;
			}
			if (!session) {
				session = new HtmlSession(source, width, height, object.timeoutMs);
				this.#htmlSessions.set(request.objectId, session);
			}
			try {
				await session.render(frame, grpcParamsToJsParams(request.parameters));
			} catch (error) {
				session.dispose();
				this.#htmlSessions.delete(request.objectId);
				throw error;
			}
			await this.#renderQueue.render(priorityLevels.render, () => {
				if (generation !== this.#generation) return "skip";
				this.drawMessage(RootRenderResponseSchema, { response: {
					case: "success",
					value: {
						isIncomplete: false,
						renderResponses: [{
							nonce: request.renderNonce,
							response: {
								case: "rendereredObjectInfo",
								value: {
									x: 0,
									y: METADATA_ROWS,
									width,
									height
								}
							}
						}]
					}
				} }, nonce);
			});
		} catch (error) {
			if (generation !== this.#generation) return;
			this.hideHtml();
			await this.#renderQueue.render(priorityLevels.render, () => {
				this.drawMessage(RootRenderResponseSchema, { response: {
					case: "errorMessage",
					value: String(error)
				} }, nonce);
			});
		}
	}
	purgeCache() {
		this.#generation++;
		if (this.#captureNonce !== 0) this.#captureNonce = null;
		for (const session of this.#htmlSessions.values()) session.dispose();
		this.#htmlSessions.clear();
		runtimeLog.info`Purging context cache (${contexts.size} contexts)`;
		for (const ctx of contexts.values()) ctx.teardown();
		contexts.clear();
		initializePromises.clear();
		this.#scheduleNotifications();
	}
	async doRender(request) {
		const object = this.objects.get(request.object);
		if (!object) {
			runtimeLog.warn`Object not found: ${request.object}`;
			return {
				type: "error",
				renderNonce: request.renderNonce,
				error: `Object not found: ${request.object}`
			};
		}
		if ("kind" in object) throw new Error("HTML cannot use the Canvas render path");
		const params = grpcParamsToJsParams(request.parameters);
		const ctx = await initializeContext(request.objectId, object, request, params);
		ctx.setFrameInfo(request.frameInfo);
		object.draw(ctx, ctx.p, params);
		const p5Canvas = ctx.mainCanvas;
		return {
			type: "success",
			renderNonce: request.renderNonce,
			width: p5Canvas.width,
			height: p5Canvas.height,
			canvas: cloneCanvas(p5Canvas.elt)
		};
	}
	#pixelDataCache = null;
	drawMessage(schema, data, nonce) {
		const message = protobuf.toBinary(schema, isMessage(data) ? data : protobuf.create(schema, data));
		this.drawRawMessage(message, nonce);
	}
	drawRawMessage(message, nonce) {
		const binaryLength = message.length;
		const payload = [
			255,
			192,
			128,
			nonce & 255,
			nonce >> 8 & 255,
			nonce >> 16 & 255,
			nonce >> 24 & 255,
			binaryLength & 255,
			binaryLength >> 8 & 255,
			binaryLength >> 16 & 255,
			binaryLength >> 24 & 255,
			...message
		];
		runtimeLog.debug`Sending message with nonce ${nonce} and length ${binaryLength}`;
		const numPixels = Math.ceil(payload.length / 3);
		const requiredHeight = Math.ceil(numPixels / this.canvas.width);
		if (requiredHeight > METADATA_ROWS) throw new Error(`Metadata rows (${METADATA_ROWS}) are not enough to draw the message (requires ${requiredHeight} rows).`);
		if (!this.#pixelDataCache) this.#pixelDataCache = this.ctx.createImageData(this.canvas.width, 64);
		const pixelData = this.#pixelDataCache;
		for (let i = 0; i < payload.length; i += 3) {
			const chunk = payload.slice(i, i + 3);
			const index = i / 3;
			const x = index % this.canvas.width;
			const pixelIndex = (Math.floor(index / this.canvas.width) * this.canvas.width + x) * 4;
			pixelData.data[pixelIndex + 0] = chunk[0] || 0;
			pixelData.data[pixelIndex + 1] = chunk[1] || 0;
			pixelData.data[pixelIndex + 2] = chunk[2] || 0;
			pixelData.data[pixelIndex + 3] = 255;
		}
		this.ctx.putImageData(pixelData, 0, 0);
	}
	pushLog(level, message) {
		this.#logQueue.push({
			level,
			message
		});
		this.#scheduleNotifications();
	}
	static get() {
		return window.__vi5__;
	}
	register(object) {
		runtimeLog.info`Registering object: ${object.id} (${object.label})`;
		this.objects.set(object.id, object);
		this.purgeCache();
		this.#notifyObjectInfos();
	}
	unregister(id) {
		runtimeLog.info`Unregistering object: ${id}`;
		this.objects.delete(id);
		this.purgeCache();
		this.#notifyObjectInfos();
	}
	get isNotifying() {
		return this.#notifyCounter.count > 0;
	}
};

//#endregion
//#region src/server/index.css?raw
var server_default = "#vi5-canvas {\n  position: fixed;\n  top: 0;\n  left: 0;\n  width: 4096px;\n  height: 2304px;\n}\ncanvas:not(#vi5-canvas) {\n  display: none;\n}\n\nbody,\nhtml {\n  background: transparent;\n  margin: 0;\n  padding: 0;\n  overflow: hidden;\n}\n";

//#endregion
//#region src/client.ts
document.head.insertAdjacentHTML("beforeend", `<style>${server_default}</style>`);
vi5Log.info("Vi5 Client Runtime initializing...");
window.__vi5__ = new Vi5Runtime(__vi5_data__.projectName);
const promises = [];
for (const objectName of __vi5_data__.objectList) {
	vi5Log.info(`Loading object module: ${objectName}`);
	promises.push(import(
		/* @vite-ignore */
		`${objectName}`
).then((module) => {
		const object = module.default;
		window.__vi5__.register(object);
	}));
}
Promise.allSettled(promises).then(() => {
	window.__vi5__.init();
	if (__vi5_data__.hookConsoleLog) hookConsole();
	vi5Log.info("Vi5 Client Runtime initialized.");
});
function hookConsole() {
	for (const { method, level } of [
		{
			method: "log",
			level: "info"
		},
		{
			method: "info",
			level: "info"
		},
		{
			method: "warn",
			level: "warn"
		},
		{
			method: "error",
			level: "error"
		}
	]) {
		const original = console[method];
		console[method] = (...args) => {
			original.apply(console, args);
			if (window.__vi5__.isNotifying) return;
			const message = args.map((arg) => {
				if (typeof arg === "string") return arg;
				try {
					return JSON.stringify(arg);
				} catch {
					return String(arg);
				}
			}).join(" ");
			window.__vi5__.pushLog(level, message);
		};
	}
}

//#endregion
export {  };