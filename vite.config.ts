import { defineConfig } from "vite";

// Tauri 开发服务器固定在 1420 端口（见 src-tauri/tauri.conf.json devUrl）。
// watch 忽略 Rust 侧源码，避免 cargo 构建触发前端热更新。
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
});
