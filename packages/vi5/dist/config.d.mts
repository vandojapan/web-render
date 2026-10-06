import { PluginOption, UserConfig } from "vite";

//#region src/config.d.ts
interface Config {
  name: string;
  vite?: UserConfig;
  vitePlugins?: PluginOption[];
  hookConsoleLog?: boolean;
}
type ConfigExport = Config | (() => Config) | Promise<Config> | (() => Promise<Config>);
declare function defineConfig(config: ConfigExport): ConfigExport;
//#endregion
export { Config, defineConfig };