#!/usr/bin/env node
import yargs from "yargs";
import { createServer } from "vite";
import path from "node:path";
import { pathToFileURL } from "node:url";
import fs from "node:fs/promises";
import picomatch from "picomatch";
import net from "node:net";
import { createJiti } from "jiti";

//#region src/server/index.html?raw
var server_default = "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"UTF-8\" />\n    <link rel=\"icon\" type=\"image/svg+xml\" href=\"/vite.svg\" />\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n    <title>vi5</title>\n  </head>\n  <body>\n    <canvas id=\"vi5-canvas\" width=\"4096\" height=\"2304\"></canvas>\n    <script type=\"module\" src=\"/@fs/!DIRNAME!/client.mjs\"><\/script>\n  </body>\n</html>\n";

//#endregion
//#region src/helpers/dedent.ts
function dedent(string) {
	const lines = string.split("\n");
	let minIndent = null;
	for (const line of lines) {
		const match = line.match(/^(\s*)\S/);
		if (match) {
			const indent = match[1].length;
			if (minIndent === null || indent < minIndent) minIndent = indent;
		}
	}
	if (minIndent !== null && minIndent > 0) return lines.map((line) => line.startsWith(" ".repeat(minIndent)) ? line.slice(minIndent) : line).join("\n");
	return string;
}

//#endregion
//#region src/server/plugin.ts
function createVi5Plugin(config, restartServer) {
	const isObjectFile = picomatch("**/*.object.ts");
	const buildObjectList = async () => Array.fromAsync(fs.glob("./src/**/*.object.ts")).then((files) => files.map((f) => "/" + f.replace(/\\/g, "/")));
	return {
		name: "vi5",
		configureServer(server) {
			server.watcher.on("add", (file) => {
				if (!isObjectFile(file)) return;
				buildObjectList().then((list) => {
					server.ws.send({
						type: "custom",
						event: "vi5:on-object-list-changed",
						data: list
					});
				});
			});
			server.watcher.on("unlink", (file) => {
				if (!isObjectFile(file)) return;
				buildObjectList().then((list) => {
					server.ws.send({
						type: "custom",
						event: "vi5:on-object-list-changed",
						data: list
					});
				});
			});
			server.middlewares.use((req, res, next) => {
				if (req.url === "/vi5") {
					res.statusCode = 200;
					res.setHeader("Content-Type", "text/html");
					res.end(server_default.replace("!DIRNAME!", import.meta.dirname));
					server.watcher.add("./src/**/*.object.ts");
					return;
				}
				next();
			});
		},
		async config() {
			return {
				server: {
					fs: { allow: [import.meta.dirname, process.cwd()] },
					hmr: { overlay: false }
				},
				optimizeDeps: { include: [
					"web-render/client",
					"web-render > @logtape/logtape",
					"web-render > fast-base64",
					"web-render > @bufbuild/protobuf",
					"web-render > @bufbuild/protobuf/codegenv2"
				] },
				define: { __vi5_data__: {
					projectName: config.name,
					objectList: await buildObjectList(),
					hookConsoleLog: config.hookConsoleLog ?? true
				} }
			};
		},
		transform: {
			filter: { id: /.*\.object\.ts$/ },
			async handler(code, _id) {
				return code + "\n" + dedent(`
        let __vi5_objectId = null;
        export const __vi5_setObjectId = (id) => (__vi5_objectId = id);
        if (import.meta.hot) {
          import.meta.hot.accept((newModule) => {
            if (newModule?.default) {
              window.__vi5__.register(newModule.default);
              newModule.__vi5_setObjectId(newModule.default.id);
            }
          });
          import.meta.hot.prune(() => {
            window.__vi5__.unregister(__vi5_objectId);
          });
        }
        `);
			}
		},
		hotUpdate({ file }) {
			if (file === "web-render.config.ts") {
				this.info("web-render.config.ts changed. restarting server...");
				restartServer();
			}
		}
	};
}

//#endregion
//#region src/helpers/port.ts
function checkPort(port) {
	return new Promise((resolve) => {
		const server = net.createServer();
		server.once("error", () => {
			resolve(false);
		});
		server.once("listening", () => {
			server.close();
			resolve(true);
		});
		server.listen(port);
	});
}
async function getUnusedPort(startingPort) {
	let port = startingPort;
	while (!await checkPort(port)) port++;
	return port;
}

//#endregion
//#region src/server/index.ts
const jiti = createJiti(import.meta.url);
async function runServer(root, port) {
	const parentPid = Number(process.env.WEB_RENDER_PARENT_PID);
	if (Number.isInteger(parentPid) && parentPid > 0) setInterval(() => {
		try {
			process.kill(parentPid, 0);
		} catch (error) {
			if (error.code === "ESRCH") process.exit(0);
		}
	}, 1e3).unref();
	let restartPromise;
	let config = await resolveConfig(root);
	const createDevServer = async (isRestart = true) => {
		const server = await createServer$1(root, port, config, restartServer);
		function restartServer() {
			if (!restartPromise) restartPromise = (async () => {
				try {
					config = await resolveConfig(root);
				} catch (err) {
					console.error(`failed to resolve config. error:`, err);
					return;
				}
				await server.close();
				await createDevServer();
			})().finally(() => {
				restartPromise = void 0;
			});
			return restartPromise;
		}
		await server.listen(void 0, isRestart);
	};
	createDevServer(false).catch((err) => {
		console.error(err);
		process.exit(1);
	});
}
async function createServer$1(root, port, config, restartServer) {
	return createServer({
		root,
		plugins: [...config.vitePlugins ?? [], createVi5Plugin(config, restartServer)],
		server: {
			port: port || await getUnusedPort(3e3),
			host: "localhost",
			strictPort: true
		}
	});
}
async function resolveConfig(root) {
	const configUrl = pathToFileURL(path.resolve(root, "web-render.config.ts"));
	const mod = await jiti.import(configUrl.href);
	const configExport = mod.default || mod;
	return typeof configExport === "function" ? configExport() : Promise.resolve(configExport);
}

//#endregion
//#region src/cli.ts
yargs(process.argv.slice(2)).command("start", "Start the server", (yargs) => {
	return yargs.option("port", {
		alias: "p",
		type: "number",
		description: "Port to run the server on",
		default: 0
	});
}, async (argv) => {
	runServer(process.cwd(), argv.port);
}).parse();

//#endregion
export {  };