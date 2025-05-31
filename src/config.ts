import { LazyStore } from "@tauri-apps/plugin-store";

export const config = new LazyStore("config.json");

export default config;
