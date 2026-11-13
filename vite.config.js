import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri sets this when developing on a device over the network (e.g. Android later).
const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [svelte()],

  // Keep Rust compiler errors visible in the terminal instead of wiping them.
  clearScreen: false,
  server: {
    // tauri.conf.json's devUrl points here, so the port must not silently change.
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // Cargo rebuilds the Rust side itself; Vite reloading on those files would be noise.
      ignored: ["**/src-tauri/**"],
    },
  },
});
