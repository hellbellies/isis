import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import ui from "@nuxt/ui/vite";
import process from "node:process";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [
    vue(),
    ui({
      // Links and navigation menus integrate with vue-router (see src/router.ts).
      router: true,
      // Composables are imported explicitly (e.g. from "@nuxt/ui/composables").
      autoImport: false,
      // Components (U*) are auto-registered; types land in components.d.ts.
      components: { dts: "src/components.d.ts" },
      // Bundle every `i-lucide-*` icon named in the source: the built app's CSP blocks
      // the Iconify API, which only the dev server (without that CSP) can reach.
      // The default scan skips `.ts` files, so name the globs explicitly.
      icon: { clientBundle: { scan: { globInclude: ["src/**/*.{vue,ts}"] } } },
      // Nuxt UI's defaults plus `epic`, which colors Jira epic badges.
      theme: { colors: ["primary", "secondary", "success", "info", "warning", "error", "epic"] },
      ui: {
        colors: { primary: "indigo", neutral: "zinc", epic: "purple" },
        // The panel body is a scrolling flex column; without this, children with
        // `overflow-hidden` (e.g. UCard) shrink to the viewport and clip instead of scrolling.
        dashboardPanel: { slots: { body: "*:shrink-0" } },
      },
    }),
  ],

  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
