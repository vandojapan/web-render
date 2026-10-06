import { fileDesc, messageDesc } from "@bufbuild/protobuf/codegenv2";
import p5 from "p5";

//#region src/gen/common_pb.ts
/**
* Describes the file common.proto.
*/
const file_common = /* @__PURE__ */ fileDesc("Cgxjb21tb24ucHJvdG8SBmNvbW1vbiIGCgRWb2lkIqoBCg1SZW5kZXJSZXF1ZXN0EhQKDHJlbmRlcl9ub25jZRgBIAEoBRIOCgZvYmplY3QYAiABKAkSEQoJb2JqZWN0X2lkGAMgASgDEiUKCmZyYW1lX2luZm8YBCABKAsyES5jb21tb24uRnJhbWVJbmZvEiUKCnBhcmFtZXRlcnMYBSADKAsyES5jb21tb24uUGFyYW1ldGVyEhIKCmlzX29mZmxpbmUYBiABKAgi7gEKCUZyYW1lSW5mbxIJCgF4GAEgASgBEgkKAXkYAiABKAESCQoBehgDIAEoARIUCgxzY3JlZW5fd2lkdGgYBCABKAUSFQoNc2NyZWVuX2hlaWdodBgFIAEoBRIVCg1jdXJyZW50X2ZyYW1lGAYgASgFEhQKDGN1cnJlbnRfdGltZRgHIAEoARIUCgx0b3RhbF9mcmFtZXMYCCABKAUSEgoKdG90YWxfdGltZRgJIAEoARIRCglmcmFtZXJhdGUYCiABKAESFAoMZ2xvYmFsX2ZyYW1lGAsgASgFEhMKC2dsb2JhbF90aW1lGAwgASgBIqABCglQYXJhbWV0ZXISCwoDa2V5GAEgASgJEhMKCXN0cl92YWx1ZRgCIAEoCUgAEhQKCnRleHRfdmFsdWUYAyABKAlIABIWCgxudW1iZXJfdmFsdWUYBCABKAFIABIUCgpib29sX3ZhbHVlGAUgASgISAASJAoLY29sb3JfdmFsdWUYBiABKAsyDS5jb21tb24uQ29sb3JIAEIHCgV2YWx1ZSIzCgVDb2xvchIJCgFyGAEgASgNEgkKAWcYAiABKA0SCQoBYhgDIAEoDRIJCgFhGAQgASgNImMKCk9iamVjdEluZm8SCgoCaWQYASABKAkSDQoFbGFiZWwYAiABKAkSOgoVcGFyYW1ldGVyX2RlZmluaXRpb25zGAMgAygLMhsuY29tbW9uLlBhcmFtZXRlckRlZmluaXRpb24iEQoPUGFyYW1ldGVyU3RyaW5nIg8KDVBhcmFtZXRlclRleHQiEgoQUGFyYW1ldGVyQm9vbGVhbiJNCg9QYXJhbWV0ZXJOdW1iZXISIAoEc3RlcBgBIAEoDjISLmNvbW1vbi5OdW1iZXJTdGVwEgsKA21pbhgCIAEoARILCgNtYXgYAyABKAEiEAoOUGFyYW1ldGVyQ29sb3Ii6gEKDVBhcmFtZXRlclR5cGUSKQoGc3RyaW5nGAEgASgLMhcuY29tbW9uLlBhcmFtZXRlclN0cmluZ0gAEiUKBHRleHQYAiABKAsyFS5jb21tb24uUGFyYW1ldGVyVGV4dEgAEisKB2Jvb2xlYW4YAyABKAsyGC5jb21tb24uUGFyYW1ldGVyQm9vbGVhbkgAEikKBm51bWJlchgEIAEoCzIXLmNvbW1vbi5QYXJhbWV0ZXJOdW1iZXJIABInCgVjb2xvchgFIAEoCzIWLmNvbW1vbi5QYXJhbWV0ZXJDb2xvckgAQgYKBGtpbmQigAEKE1BhcmFtZXRlckRlZmluaXRpb24SCwoDa2V5GAEgASgJEiMKBHR5cGUYAiABKAsyFS5jb21tb24uUGFyYW1ldGVyVHlwZRINCgVsYWJlbBgDIAEoCRIoCg1kZWZhdWx0X3ZhbHVlGAQgASgLMhEuY29tbW9uLlBhcmFtZXRlciJEChJCYXRjaFJlbmRlclJlcXVlc3QSLgoPcmVuZGVyX3JlcXVlc3RzGAEgAygLMhUuY29tbW9uLlJlbmRlclJlcXVlc3QqgQEKCk51bWJlclN0ZXASEwoPTlVNQkVSX1NURVBfT05FEAASGQoVTlVNQkVSX1NURVBfUE9JTlRfT05FEAESHgoaTlVNQkVSX1NURVBfUE9JTlRfWkVST19PTkUQAhIjCh9OVU1CRVJfU1RFUF9QT0lOVF9aRVJPX1pFUk9fT05FEANiBnByb3RvMw");
/**
* Describes the message common.Parameter.
* Use `create(ParameterSchema)` to create a new message.
*/
const ParameterSchema = /* @__PURE__ */ messageDesc(file_common, 3);
/**
* Describes the message common.ObjectInfo.
* Use `create(ObjectInfoSchema)` to create a new message.
*/
const ObjectInfoSchema = /* @__PURE__ */ messageDesc(file_common, 5);
/**
* Describes the message common.ParameterType.
* Use `create(ParameterTypeSchema)` to create a new message.
*/
const ParameterTypeSchema = /* @__PURE__ */ messageDesc(file_common, 11);
/**
* Describes the message common.ParameterDefinition.
* Use `create(ParameterDefinitionSchema)` to create a new message.
*/
const ParameterDefinitionSchema = /* @__PURE__ */ messageDesc(file_common, 12);
/**
* Describes the message common.BatchRenderRequest.
* Use `create(BatchRenderRequestSchema)` to create a new message.
*/
const BatchRenderRequestSchema = /* @__PURE__ */ messageDesc(file_common, 13);
/**
* @generated from enum common.NumberStep
*/
let NumberStep = /* @__PURE__ */ function(NumberStep) {
	/**
	* @generated from enum value: NUMBER_STEP_ONE = 0;
	*/
	NumberStep[NumberStep["ONE"] = 0] = "ONE";
	/**
	* @generated from enum value: NUMBER_STEP_POINT_ONE = 1;
	*/
	NumberStep[NumberStep["POINT_ONE"] = 1] = "POINT_ONE";
	/**
	* @generated from enum value: NUMBER_STEP_POINT_ZERO_ONE = 2;
	*/
	NumberStep[NumberStep["POINT_ZERO_ONE"] = 2] = "POINT_ZERO_ONE";
	/**
	* @generated from enum value: NUMBER_STEP_POINT_ZERO_ZERO_ONE = 3;
	*/
	NumberStep[NumberStep["POINT_ZERO_ZERO_ONE"] = 3] = "POINT_ZERO_ZERO_ONE";
	return NumberStep;
}({});

//#endregion
//#region src/user/context.ts
p5.disableFriendlyErrors = true;
function setProperty(p, property, value) {
	p[property] = value;
}
var Vi5Context = class {
	#p5Instance = null;
	#mainCanvas = null;
	#graphics = [];
	#frameInfo = null;
	/** @internal */
	constructor() {
		this.#p5Instance = null;
	}
	/** @internal */
	initialize(p5Instance) {
		this.#p5Instance = p5Instance;
	}
	get p() {
		if (!this.#p5Instance) throw new Error("p5 instance has not been initialized yet.");
		return this.#p5Instance;
	}
	createCanvas(width, height, renderer) {
		this.#mainCanvas = this.p.createCanvas(width, height, renderer);
		return this.#mainCanvas;
	}
	createGraphics(width, height, renderer) {
		const newGraphics = this.p.createGraphics(width, height, renderer);
		this.#graphics.push(newGraphics);
		return newGraphics;
	}
	get mainCanvas() {
		if (!this.#mainCanvas) throw new Error("Main canvas has not been created yet.");
		return this.#mainCanvas;
	}
	get frameInfo() {
		if (!this.#frameInfo) throw new Error("Frame info has not been set yet.");
		return this.#frameInfo;
	}
	/** @internal */
	setFrameInfo(frameInfo) {
		this.#frameInfo = frameInfo;
		setProperty(this.p, "frameCount", frameInfo.currentFrame);
		setProperty(this.p, "deltaTime", 1e3 / frameInfo.framerate);
	}
	notify(level, message) {
		window.__vi5__.pushLog(level, message);
	}
	/** @internal */
	teardown() {
		this.#mainCanvas?.remove();
		this.#graphics.forEach((g) => g.remove());
		this.#graphics = [];
	}
};

//#endregion
export { ParameterDefinitionSchema as a, file_common as c, ObjectInfoSchema as i, BatchRenderRequestSchema as n, ParameterSchema as o, NumberStep as r, ParameterTypeSchema as s, Vi5Context as t };