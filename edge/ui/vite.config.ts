import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Vite config for the Wagner Edge desktop frontend. Tauri serves the built
// `dist/` in production and proxies the dev server (fixed port 1420) in
// `tauri dev`. `base: "./"` keeps asset URLs relative so they resolve under
// the `tauri://localhost` webview origin.
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/shell/**"] },
  },
  build: {
    outDir: "dist",
    target: "es2022",
    sourcemap: true,
  },
});
