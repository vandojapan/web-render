/// <reference types="vite/client" />
import * as fastBase64 from "fast-base64";
import * as protobuf from "@bufbuild/protobuf";
import {
  BatchRenderRequestSchema,
  type ObjectInfo,
  type Parameter,
  type ParameterDefinition as GrpcParameterDefinition,
  type ParameterType as GrpcParameterType,
  type RenderRequest,
  ParameterDefinitionSchema as GrpcParameterDefinitionSchema,
  ParameterTypeSchema,
  ParameterSchema,
  ObjectInfoSchema,
} from "../gen/common_pb";

import { vi5Log } from "./log";
import {
  InitializeInfoSchema,
  RootRenderResponseSchema,
  type MaybeIncompleteRenderResponse,
  type RendereredObjectInfo,
  LogLevel,
  NotificationsSchema,
} from "../gen/server-js_pb";
import type {
  InferParameters,
  ParameterDefinitions,
  ParameterType,
  Vi5Object,
} from "../user/object";
import { Vi5Context } from "../user/context";
import { packCanvases, type JsRenderResponse } from "./packCanvas";
import p5 from "p5";
import type { HtmlObject } from "../user/html";
import { HtmlSession, METADATA_ROWS } from "./htmlSession.mjs";
import { DisposableCounterFactory } from "./disposableCounter";
import { priorityLevels, RenderQueue } from "./renderQueue";

const runtimeLog = vi5Log.getChild("Vi5Runtime");
type NotificationLevelKey = keyof typeof notificationLevelMap;
const notificationNonce = 1;
const notificationLevelMap = {
  info: LogLevel.INFO,
  warn: LogLevel.WARN,
  error: LogLevel.ERROR,
} as const;

const isMessage = <Desc extends protobuf.DescMessage>(
  data: protobuf.MessageShape<Desc> | protobuf.MessageInitShape<Desc>,
): data is protobuf.MessageShape<Desc> => {
  return typeof data === "object" && data !== null && "$typeName" in data;
};

// const initializePromises: Record<bigint, Promise<void>> = {};
const initializePromises = new Map<bigint, Promise<Vi5Context>>();
const contexts = new Map<bigint, Vi5Context>();

async function initializeContext<T extends ParameterDefinitions>(
  id: bigint,
  object: Vi5Object<T>,
  renderRequest: RenderRequest,
  parameter: InferParameters<T>,
): Promise<Vi5Context> {
  if (contexts.has(id)) {
    return contexts.get(id)!;
  }
  if (initializePromises.has(id)) {
    return initializePromises.get(id)!;
  }
  const ctx = new Vi5Context();
  const { promise, resolve, reject } = Promise.withResolvers<Vi5Context>();
  initializePromises.set(id, promise);
  new p5((sketch) => {
    ctx.initialize(sketch);
    // Let p5 perform its own initialization before calling the object setup.
    // Calling setup here manually runs it twice and can replace the renderer
    // after an async setup has already created the main canvas.
    sketch.setup = async () => {
      sketch.noLoop();
      ctx.setFrameInfo(renderRequest.frameInfo!);
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
function grpcParamsToJsParams<T extends ParameterDefinitions>(
  grpcParams: Parameter[],
): InferParameters<T> {
  const params: Record<string, ParameterType<any>> = {};
  for (const param of grpcParams) {
    switch (param.value.case) {
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
          a: param.value.value.a as 0 | 255,
        };
        break;
      default:
        runtimeLog.warn`Unknown parameter value case: ${param.value.case satisfies undefined}`;
    }
  }
  return params as InferParameters<T>;
}

function toGrpcParameterType(
  definition: ParameterDefinitions[string],
): GrpcParameterType {
  switch (definition.type) {
    case "string":
      return protobuf.create(ParameterTypeSchema, {
        kind: {
          case: "string",
          value: {},
        },
      });
    case "text":
      return protobuf.create(ParameterTypeSchema, {
        kind: {
          case: "text",
          value: {},
        },
      });
    case "boolean":
      return protobuf.create(ParameterTypeSchema, {
        kind: {
          case: "boolean",
          value: {},
        },
      });
    case "number":
      return protobuf.create(ParameterTypeSchema, {
        kind: {
          case: "number",
          value: {
            step: definition.step,
            min: definition.min,
            max: definition.max,
          },
        },
      });
    case "color":
      return protobuf.create(ParameterTypeSchema, {
        kind: {
          case: "color",
          value: {},
        },
      });
  }
}

function toGrpcDefaultValue(
  key: string,
  definition: ParameterDefinitions[string],
): Parameter | undefined {
  if (definition.default === undefined) {
    return undefined;
  }
  switch (definition.type) {
    case "string":
      return protobuf.create(ParameterSchema, {
        key,
        value: { case: "strValue", value: definition.default },
      });
    case "text":
      return protobuf.create(ParameterSchema, {
        key,
        value: { case: "textValue", value: definition.default },
      });
    case "number":
      return protobuf.create(ParameterSchema, {
        key,
        value: { case: "numberValue", value: definition.default },
      });
    case "boolean":
      return protobuf.create(ParameterSchema, {
        key,
        value: { case: "boolValue", value: definition.default },
      });
    case "color":
      return protobuf.create(ParameterSchema, {
        key,
        value: {
          case: "colorValue",
          value: {
            r: definition.default.r,
            g: definition.default.g,
            b: definition.default.b,
            a: definition.default.a,
          },
        },
      });
  }
}

function toGrpcParameterDefinition(
  key: string,
  definition: ParameterDefinitions[string],
): GrpcParameterDefinition {
  return protobuf.create(GrpcParameterDefinitionSchema, {
    key,
    label: definition.label ?? key,
    type: toGrpcParameterType(definition),
    defaultValue: toGrpcDefaultValue(key, definition),
  });
}

const temporaryCanvases: HTMLCanvasElement[] = [];

const cloneCanvas = (source: HTMLCanvasElement): HTMLCanvasElement => {
  if (temporaryCanvases.length === 0) {
    runtimeLog.debug`Adding new temporary canvas (pool size: ${temporaryCanvases.length + 1})`;
  }
  const clone = temporaryCanvases.pop() ?? document.createElement("canvas");
  if (clone.width < source.width) {
    runtimeLog.debug`Resizing temporary canvas to width ${source.width}`;
    clone.width = source.width;
  }
  if (clone.height < source.height) {
    runtimeLog.debug`Resizing temporary canvas to height ${source.height}`;
    clone.height = source.height;
  }
  const ctx = clone.getContext("2d")!;
  ctx.clearRect(0, 0, source.width, source.height);
  ctx.drawImage(source, 0, 0, source.width, source.height);
  return clone;
};

const disposeCanvas = (canvas: HTMLCanvasElement): void => {
  temporaryCanvases.push(canvas);
};

export class Vi5Runtime {
  readonly canvas: HTMLCanvasElement;
  readonly ctx: CanvasRenderingContext2D;
  readonly objects = new Map<string, Vi5Object<ParameterDefinitions> | HtmlObject>();
  #htmlSessions = new Map<bigint, HtmlSession>();
  // Initialization uses the same capture lease as frame responses. Notifications
  // must not replace nonce 0 before CEF's first paint has consumed it.
  #captureNonce: number | null = 0;
  #generation = 0;
  #objectInfosDirty = false;
  #notificationsScheduled = false;
  #renderQueue = new RenderQueue();
  #notifyCounter = new DisposableCounterFactory();
  #logQueue: { level: NotificationLevelKey; message: string }[] = [];

  constructor(public projectName: string) {
    this.canvas = document.getElementById("vi5-canvas") as HTMLCanvasElement;
    this.ctx = this.canvas.getContext("2d")!;
  }

  async init() {
    await this.#renderQueue.render(priorityLevels.init, () => {
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
      this.drawMessage(
        InitializeInfoSchema,
        {
          projectName: this.projectName,
          rendererVersion: "1.0.0",
        },
        0,
      );
    });
    this.#notifyObjectInfos();

    if (import.meta.hot) {
      import.meta.hot.on("vi5:on-object-list-changed", (_list) => {
        // TOOD: オブジェクトの追加・削除に対応する
        runtimeLog.info`Object list changed, reloading page...`;
        window.location.reload();
      });
    }
  }

  #notifyObjectInfos() {
    this.#objectInfosDirty = true;
    this.#scheduleNotifications();
  }

  #scheduleNotifications() {
    if (this.#notificationsScheduled || this.#captureNonce !== null ||
        (!this.#objectInfosDirty && this.#logQueue.length === 0)) return;
    this.#notificationsScheduled = true;
    void this.#renderQueue.render(priorityLevels.notify, () => {
      this.#notificationsScheduled = false;
      if (this.#captureNonce !== null) return "skip";
      const counter = this.#notifyCounter.createCounter();
      try {
      const objectInfos = Array.from(this.objects.values()).map(
        (obj): ObjectInfo =>
          protobuf.create(ObjectInfoSchema, {
            id: obj.id,
            label: obj.label,
            parameterDefinitions: Object.entries(obj.parameters).map(
              ([key, def]) => toGrpcParameterDefinition(key, def),
            ),
          }),
      );
      this.drawMessage(
        NotificationsSchema,
        {
          entries: [
            ...(this.#objectInfosDirty ? [{
              entry: {
                case: "objectListUpdate" as const,
                value: {
                  objectInfos,
                },
              },
            }] : []),
            ...this.#logQueue.map(({level, message}) => ({ entry: {
              case: "log" as const, value: { level: notificationLevelMap[level], message },
            }})),
          ],
        },
        notificationNonce,
      );
      this.#objectInfosDirty = false;
      this.#logQueue.length = 0;
      } finally { counter[Symbol.dispose](); }
    });
  }

  /** Called by CEF only after it has copied the captured RGBA for this nonce. */
  acknowledge(nonce: number) {
    if (this.#captureNonce !== nonce) return;
    this.#captureNonce = null;
    this.#scheduleNotifications();
  }

  cancelCapture(nonce: number) {
    if (this.#captureNonce === nonce) this.purgeCache();
  }

  async render(nonce: number, dataB64: string) {
    if (this.#captureNonce !== null) throw new Error("Previous native capture has not been acknowledged");
    this.#captureNonce = nonce;
    const generation = this.#generation;
    try {
      const data = await fastBase64.toBytes(dataB64);
      const renderPayload = protobuf.fromBinary(BatchRenderRequestSchema, data);
      // The native server splits batches and awaits each CEF capture before requesting the next.
      if (renderPayload.renderRequests.some(req => "kind" in (this.objects.get(req.object) ?? {}))) {
        if (renderPayload.renderRequests.length !== 1) throw new Error("HTML requires single-frame capture");
        const req = renderPayload.renderRequests[0]!;
        await this.renderHtml(nonce, req, generation);
        return;
      }
      this.hideHtml();
      const jsResponses: JsRenderResponse[] = [];

      for (const req of renderPayload.renderRequests) {
        try {
          jsResponses.push(await this.doRender(req));
        } catch (e) {
          runtimeLog.error`Error during rendering object ${req.object}: ${String(e)}`;
          jsResponses.push({
            type: "error",
            renderNonce: req.renderNonce,
            error: `Error during rendering: ${String(e)}`,
          });
        }
      }
      const canvases = new Map<number, HTMLCanvasElement>();
      for (const resp of jsResponses) {
        if (resp.type === "success") {
          canvases.set(resp.renderNonce, resp.canvas);
        }
      }
      const packed = packCanvases(jsResponses);
      for (const packedResponse of packed) {
        await this.#renderQueue.render(priorityLevels.render, () => {
          if (generation === this.#generation) this.renderSingleResponse(packedResponse, nonce, canvases);
        });
      }
      for (const canvas of canvases.values()) {
        disposeCanvas(canvas);
      }
      canvases.clear();
    } catch (e) {
      if (generation !== this.#generation) return;
      runtimeLog.error`Error during batch rendering: ${String(e)}`;
      this.drawMessage(
        RootRenderResponseSchema,
        {
          response: {
            case: "errorMessage",
            value: `Error during batch rendering: ${String(e)}`,
          },
        },
        nonce,
      );
    }
  }

  private renderSingleResponse(
    packedResponse: MaybeIncompleteRenderResponse,
    nonce: number,
    canvases: Map<number, HTMLCanvasElement>,
  ) {
    this.drawMessage(
      RootRenderResponseSchema,
      {
        response: {
          case: "success",
          value: packedResponse,
        },
      },
      nonce,
    );
    for (const renderResponse of packedResponse.renderResponses) {
      if (renderResponse.response.case === "rendereredObjectInfo") {
        const info = renderResponse.response.value as RendereredObjectInfo;
        this.ctx.clearRect(info.x, info.y, info.width, info.height);
        this.ctx.drawImage(
          canvases.get(renderResponse.nonce)!,
          0,
          0,
          info.width,
          info.height,
          info.x,
          info.y,
          info.width,
          info.height,
        );
        runtimeLog.debug`Rendered object ${renderResponse.nonce} at (${info.x}, ${info.y}) with size ${info.width}x${info.height}`;
      }
    }
  }

  private hideHtml() {
    for (const session of this.#htmlSessions.values()) session.hide();
  }

  private async renderHtml(nonce: number, request: RenderRequest, generation: number) {
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
        session = undefined;
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
        this.drawMessage(RootRenderResponseSchema, {
          response: { case: "success", value: {
            isIncomplete: false,
            renderResponses: [{ nonce: request.renderNonce, response: {
              case: "rendereredObjectInfo",
              value: { x: 0, y: METADATA_ROWS, width, height },
            }}],
          }},
        }, nonce);
      });
    } catch (error) {
      if (generation !== this.#generation) return;
      this.hideHtml();
      await this.#renderQueue.render(priorityLevels.render, () => {
        this.drawMessage(RootRenderResponseSchema, {
          response: { case: "errorMessage", value: String(error) },
        }, nonce);
      });
    }
  }

  purgeCache() {
    this.#generation++;
    // Object registration also purges caches during startup. Preserve the
    // initialization lease until the native reader has acknowledged nonce 0.
    if (this.#captureNonce !== 0) this.#captureNonce = null;
    for (const session of this.#htmlSessions.values()) session.dispose();
    this.#htmlSessions.clear();
    runtimeLog.info`Purging context cache (${contexts.size} contexts)`;
    for (const ctx of contexts.values()) {
      ctx.teardown();
    }
    contexts.clear();
    initializePromises.clear();
    this.#scheduleNotifications();
  }

  private async doRender(request: RenderRequest): Promise<JsRenderResponse> {
    const object = this.objects.get(request.object);
    if (!object) {
      runtimeLog.warn`Object not found: ${request.object}`;
      return {
        type: "error",
        renderNonce: request.renderNonce,
        error: `Object not found: ${request.object}`,
      };
    }

    if ("kind" in object) throw new Error("HTML cannot use the Canvas render path");
    const params = grpcParamsToJsParams(request.parameters);
    // A preview must await the same setup promise as an offline render. A
    // temporary error here becomes a cached blank scene frame in AviUtl2.
    const ctx = await initializeContext(request.objectId, object, request, params);
    ctx.setFrameInfo(request.frameInfo!);
    object.draw(ctx, ctx.p, params);
    const p5Canvas = ctx.mainCanvas;
    return {
      type: "success",
      renderNonce: request.renderNonce,
      width: p5Canvas.width,
      height: p5Canvas.height,
      // TODO: 描画 -> メインキャンバスにコピー -> 次の描画、の方が速そうなのでそうする
      canvas: cloneCanvas(p5Canvas.elt),
    };
  }

  #pixelDataCache: ImageData | null = null;
  drawMessage<Desc extends protobuf.DescMessage>(
    schema: Desc,
    data: protobuf.MessageShape<Desc> | protobuf.MessageInitShape<Desc>,
    nonce: number,
  ): void {
    const message = protobuf.toBinary(
      schema,
      isMessage(data) ? data : protobuf.create(schema, data),
    );
    this.drawRawMessage(message, nonce);
  }

  drawRawMessage(message: Uint8Array, nonce: number): void {
    const binaryLength = message.length;
    const payload = [
      255,
      192,
      128,
      nonce & 0xff,
      (nonce >> 8) & 0xff,
      (nonce >> 16) & 0xff,
      (nonce >> 24) & 0xff,
      binaryLength & 0xff,
      (binaryLength >> 8) & 0xff,
      (binaryLength >> 16) & 0xff,
      (binaryLength >> 24) & 0xff,
      ...message,
    ];
    runtimeLog.debug`Sending message with nonce ${nonce} and length ${binaryLength}`;
    const numPixels = Math.ceil(payload.length / 3);
    const requiredHeight = Math.ceil(numPixels / this.canvas.width);
    if (requiredHeight > METADATA_ROWS) {
      // TODO: ちゃんとエラー処理
      throw new Error(
        `Metadata rows (${METADATA_ROWS}) are not enough to draw the message (requires ${requiredHeight} rows).`,
      );
    }

    if (!this.#pixelDataCache) {
      this.#pixelDataCache = this.ctx.createImageData(this.canvas.width, 64);
    }

    const pixelData = this.#pixelDataCache;
    for (let i = 0; i < payload.length; i += 3) {
      const chunk = payload.slice(i, i + 3);
      const index = i / 3;
      const x = index % this.canvas.width;
      const y = Math.floor(index / this.canvas.width);
      const pixelIndex = (y * this.canvas.width + x) * 4;
      pixelData.data[pixelIndex + 0] = chunk[0] || 0;
      pixelData.data[pixelIndex + 1] = chunk[1] || 0;
      pixelData.data[pixelIndex + 2] = chunk[2] || 0;
      pixelData.data[pixelIndex + 3] = 255;
    }
    this.ctx.putImageData(pixelData, 0, 0);
  }

  pushLog(level: NotificationLevelKey, message: string) {
    this.#logQueue.push({ level, message });
    this.#scheduleNotifications();
  }

  static get() {
    return window.__vi5__;
  }

  register<T extends Vi5Object<ParameterDefinitions> | HtmlObject>(object: T) {
    runtimeLog.info`Registering object: ${object.id} (${object.label})`;
    this.objects.set(object.id, object);

    this.purgeCache();

    this.#notifyObjectInfos();
  }
  unregister(id: string) {
    runtimeLog.info`Unregistering object: ${id}`;
    this.objects.delete(id);
    this.purgeCache();

    this.#notifyObjectInfos();
  }

  get isNotifying() {
    return this.#notifyCounter.count > 0;
  }
}
