import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import autoprefixer from "autoprefixer";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  css: {
    postcss: {
      plugins: [autoprefixer()],
    },
  },

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // esbuild: {
  //   drop: ["console", "debugger"],
  // },

  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules")) {
            const modulePath = id.split("node_modules/")[1];
            const topLevelFolder = modulePath?.split("/")[0];
            if (topLevelFolder !== ".pnpm") {
              return topLevelFolder;
            }

            const scopedPackageName = modulePath?.split("/")[1];
            const chunkName =
              scopedPackageName?.split("@")[
                scopedPackageName.startsWith("@") ? 1 : 0
              ];

            return chunkName;
          }
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
});
