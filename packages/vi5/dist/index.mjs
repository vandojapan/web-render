import { r as NumberStep, t as Vi5Context } from "./context-BmtDCMcB.mjs";

//#region src/user/html.ts
function defineHtmlObject(options) {
	return {
		...options,
		kind: "html"
	};
}

//#endregion
//#region src/user/object.ts
const numberStep = {
	"1": NumberStep.ONE,
	"0.1": NumberStep.POINT_ONE,
	"0.01": NumberStep.POINT_ZERO_ONE,
	"0.001": NumberStep.POINT_ZERO_ZERO_ONE
};

//#endregion
//#region src/user/utils.ts
function colorToP5Tuple(color) {
	return [
		color.r,
		color.g,
		color.b,
		color.a
	];
}

//#endregion
//#region src/index.ts
function defineObject(option) {
	return option;
}

//#endregion
export { Vi5Context, colorToP5Tuple, defineHtmlObject, defineObject, numberStep };