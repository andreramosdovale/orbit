// @ts-expect-error type error without @types/node package
import process from "node:process";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const host = process.env.TAURI_DEV_HOST;
// `npm run dev:app` escolhe uma porta livre; sem ele, a padrão.
const port = Number(process.env.ORBIT_DEV_PORT ?? 1420);

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react()],

  // `globals` faz a Testing Library limpar a tela sozinha entre os testes.
  test: {
    environment: "jsdom",
    globals: true,
    unstubGlobals: true,
    setupFiles: ["./src/test/setup.ts"],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: port + 1,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
