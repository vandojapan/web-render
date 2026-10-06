import p5 from "p5";
import { Message } from "@bufbuild/protobuf";

//#region src/gen/common_pb.d.ts
/**
 * @generated from message common.FrameInfo
 */
type FrameInfo = Message<"common.FrameInfo"> & {
  /**
   * @generated from field: double x = 1;
   */
  x: number;
  /**
   * @generated from field: double y = 2;
   */
  y: number;
  /**
   * @generated from field: double z = 3;
   */
  z: number;
  /**
   * @generated from field: int32 screen_width = 4;
   */
  screenWidth: number;
  /**
   * @generated from field: int32 screen_height = 5;
   */
  screenHeight: number;
  /**
   * @generated from field: int32 current_frame = 6;
   */
  currentFrame: number;
  /**
   * @generated from field: double current_time = 7;
   */
  currentTime: number;
  /**
   * @generated from field: int32 total_frames = 8;
   */
  totalFrames: number;
  /**
   * @generated from field: double total_time = 9;
   */
  totalTime: number;
  /**
   * @generated from field: double framerate = 10;
   */
  framerate: number;
  /**
   * @generated from field: int32 global_frame = 11;
   */
  globalFrame: number;
  /**
   * @generated from field: double global_time = 12;
   */
  globalTime: number;
};
/**
 * @generated from enum common.NumberStep
 */
declare enum NumberStep {
  /**
   * @generated from enum value: NUMBER_STEP_ONE = 0;
   */
  ONE = 0,
  /**
   * @generated from enum value: NUMBER_STEP_POINT_ONE = 1;
   */
  POINT_ONE = 1,
  /**
   * @generated from enum value: NUMBER_STEP_POINT_ZERO_ONE = 2;
   */
  POINT_ZERO_ONE = 2,
  /**
   * @generated from enum value: NUMBER_STEP_POINT_ZERO_ZERO_ONE = 3;
   */
  POINT_ZERO_ZERO_ONE = 3
}
//#endregion
//#region src/user/context.d.ts
declare class Vi5Context {
  #private;
  /** @internal */
  constructor();
  /** @internal */
  initialize(p5Instance: p5): void;
  get p(): p5;
  createCanvas(width: number, height: number, renderer?: typeof p5.P2D | typeof p5.WEBGL): p5.Renderer;
  createGraphics(width: number, height: number, renderer?: typeof p5.P2D | typeof p5.WEBGL): p5.Graphics;
  get mainCanvas(): p5.Renderer;
  get frameInfo(): FrameInfo;
  /** @internal */
  setFrameInfo(frameInfo: FrameInfo): void;
  notify(level: "info" | "warn" | "error", message: string): void;
  /** @internal */
  teardown(): void;
}
//#endregion
//#region src/user/object.d.ts
declare const parameterTypes: {
  readonly string: "string";
  readonly text: "text";
  readonly number: "number";
  readonly boolean: "boolean";
  readonly color: "color";
};
type Color = {
  r: number;
  g: number;
  b: number;
  a: 0 | 255;
};
type ParameterType<T extends keyof typeof parameterTypes> = T extends "string" ? string : T extends "text" ? string : T extends "number" ? number : T extends "boolean" ? boolean : T extends "color" ? Color : never;
declare const numberStep: {
  readonly "1": NumberStep.ONE;
  readonly "0.1": NumberStep.POINT_ONE;
  readonly "0.01": NumberStep.POINT_ZERO_ONE;
  readonly "0.001": NumberStep.POINT_ZERO_ZERO_ONE;
};
type ParameterDefinition<T extends keyof typeof parameterTypes> = T extends "number" ? {
  type: T;
  label: string;
  default: ParameterType<T>;
  step: NumberStep;
  min: number;
  max: number;
} : {
  type: T;
  label: string;
  default: ParameterType<T>;
};
type InferParameters<T extends Record<string, ParameterDefinition<keyof typeof parameterTypes>>> = { [K in keyof T]: ParameterType<T[K]["type"]> };
type ParameterDefinitions = Record<string, ParameterDefinition<keyof typeof parameterTypes>>;
type Vi5Object<T extends ParameterDefinitions> = {
  id: string;
  label: string;
  parameters: T;
  setup: (ctx: Vi5Context, p: p5, params: InferParameters<T>) => Promise<p5.Renderer> | p5.Renderer;
  draw: (ctx: Vi5Context, p: p5, params: InferParameters<T>) => void;
};
//#endregion
//#region src/user/html.d.ts
/** A local project page. CSS/SVG and an optional window.aviutl.render hook are frame driven. */
interface HtmlObject<T extends ParameterDefinitions = ParameterDefinitions> {
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
declare function defineHtmlObject<T extends ParameterDefinitions>(options: Omit<HtmlObject<T>, "kind">): HtmlObject<T>;
//#endregion
//#region src/user/utils.d.ts
declare function colorToP5Tuple(color: {
  r: number;
  g: number;
  b: number;
  a: number;
}): [number, number, number, number];
//#endregion
//#region src/index.d.ts
declare function defineObject<T extends ParameterDefinitions>(option: Vi5Object<T>): Vi5Object<T>;
//#endregion
export { type HtmlObject, Vi5Context, colorToP5Tuple, defineHtmlObject, defineObject, numberStep };