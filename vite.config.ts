import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // O Vite 8 resolve os paths do tsconfig nativamente (substitui vite-tsconfig-paths).
  resolve: {
    tsconfigPaths: true,
  },
  clearScreen: false,
  server: {
    // Porta fixa: o Tauri assume o dev server em localhost:1420 (tauri.conf.json).
    port: 1420,
    strictPort: true,
    watch: {
      // Evita que o watcher do Vite reinicie ao tocar em src-tauri/target.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Tauri injeta TAURI_ENV_* e afins; VITE_ cobre o front-end.
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "es2022",
    // Vite 8 usa rolldown/oxc — nao passar "esbuild" aqui (nao e mais dependencia do Vite).
    sourcemap: false,
  },
});
