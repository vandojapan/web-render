import { createServer as createViteServer } from "vite";
import type { Config } from "../config";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { createVi5Plugin } from "./plugin";
import { getUnusedPort } from "../helpers/port";
import { createJiti } from "jiti";

const jiti = createJiti(import.meta.url);

export async function runServer(root: string, port: number) {
  // CEF can be force-terminated without running its Rust destructors.
  // Its private Vite child must still exit instead of keeping a port alive.
  const parentPid = Number(process.env.WEB_RENDER_PARENT_PID);
  if (Number.isInteger(parentPid) && parentPid > 0) {
    setInterval(() => {
      try { process.kill(parentPid, 0); }
      catch (error: any) { if (error.code === "ESRCH") process.exit(0); }
    }, 1000).unref();
  }
  let restartPromise: Promise<void> | undefined;
  let config: Config = await resolveConfig(root);
  const createDevServer = async (isRestart = true) => {
    const server = await createServer(root, port, config, restartServer);
    function restartServer() {
      if (!restartPromise) {
        restartPromise = (async () => {
          try {
            config = await resolveConfig(root);
          } catch (err: any) {
            console.error(`failed to resolve config. error:`, err);
            return;
          }
          await server.close();
          await createDevServer();
        })().finally(() => {
          restartPromise = undefined;
        });
      }
      return restartPromise;
    }
    await server.listen(undefined, isRestart);
  };
  createDevServer(false).catch((err) => {
    console.error(err);
    process.exit(1);
  });
}

async function createServer(
  root: string,
  port: number,
  config: Config,
  restartServer: () => Promise<void>,
) {
  const userPlugins = config.vitePlugins ?? [];
  return createViteServer({
    root,
    plugins: [...userPlugins, createVi5Plugin(config, restartServer)],
    server: {
      port: port || (await getUnusedPort(3000)),
      host: "localhost",
      strictPort: true,
    },
  });
}

async function resolveConfig(root: string): Promise<Config> {
  const configPath = path.resolve(root, "web-render.config.ts");
  const configUrl = pathToFileURL(configPath);
  const mod = await jiti.import<any>(configUrl.href);
  const configExport = mod.default || mod;
  return typeof configExport === "function" ? configExport() : Promise.resolve(configExport);
}
